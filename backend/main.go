package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"
)

const (
	defaultListenAddr    = "0.0.0.0:8080"
	defaultPublicBaseURL = "http://localhost:8080"
	defaultVersion       = "1.0.0-dev"
	defaultWasmFile      = "engine.wasm"
	defaultIconsFile     = "icons.zip"
	defaultEffectsFile   = "effects_manifest.json"
	defaultWebUIURL      = "http://localhost:5173"
	defaultSignatureFile = "engine.wasm.sig.b64"
	defaultRateLimitRPS  = 60.0
	defaultRateLimitBurst = 120.0
	defaultRateLimitTTL  = 2 * time.Minute
	defaultCORSOrigins   = "http://localhost:5173,https://vst.tonelab.dev,https://tonelab.dev,https://*.vercel.app"
)

type syncAssets struct {
	IconsURL   string `json:"icons_url"`
	WebUIURL   string `json:"web_ui_url"`
	EffectsURL string `json:"effects_url"`
}

type syncResponse struct {
	Version   string     `json:"version"`
	WasmURL   string     `json:"wasm_url"`
	Signature string     `json:"signature"`
	Assets    syncAssets `json:"assets"`
}

type serverConfig struct {
	ListenAddr        string
	PublicBaseURL     string
	Version           string
	WasmFile          string
	IconsFile         string
	EffectsFile       string
	WebUIURL          string
	AssetsDir         string
	SignatureOverride string
	SignatureFilePath string
	RateLimitRPS      float64
	RateLimitBurst    float64
	RateLimitTTL      time.Duration
	CORSOrigins       []string
}

func main() {
	cfg, err := loadConfig()
	if err != nil {
		log.Fatalf("failed to load config: %v", err)
	}

	mux := http.NewServeMux()
	limiter := newRateLimiter(cfg.RateLimitRPS, cfg.RateLimitBurst, cfg.RateLimitTTL)
	mux.HandleFunc("/vst/sync", func(w http.ResponseWriter, r *http.Request) {
		setCORSHeaders(w, r, cfg)
		setSecurityHeaders(w)
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		signature, sigErr := cfg.resolveSignatureB64()
		if sigErr != nil {
			log.Printf("warning: failed to load wasm signature: %v", sigErr)
		}

		resp := syncResponse{
			Version:   cfg.Version,
			WasmURL:   fmt.Sprintf("%s/assets/%s", cfg.PublicBaseURL, cfg.WasmFile),
			Signature: signature,
			Assets: syncAssets{
				IconsURL:   fmt.Sprintf("%s/assets/%s", cfg.PublicBaseURL, cfg.IconsFile),
				WebUIURL:   cfg.WebUIURL,
				EffectsURL: fmt.Sprintf("%s/assets/%s", cfg.PublicBaseURL, cfg.EffectsFile),
			},
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(resp)
	})

	assets := http.StripPrefix("/assets/", http.FileServer(http.Dir(cfg.AssetsDir)))
	mux.Handle("/assets/", withRateLimit(withCORS(assets, cfg), limiter))

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		setCORSHeaders(w, r, cfg)
		setSecurityHeaders(w)
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"ok":true}`))
	})

	handler := withRateLimit(mux, limiter)

	server := &http.Server{
		Addr:              cfg.ListenAddr,
		Handler:           handler,
		ReadTimeout:       10 * time.Second,
		ReadHeaderTimeout: 5 * time.Second,
		WriteTimeout:      15 * time.Second,
		IdleTimeout:       60 * time.Second,
		MaxHeaderBytes:    1 << 20,
	}

	log.Printf("Tonelab Evergreen backend listening on http://%s", cfg.ListenAddr)
	log.Printf("Assets directory: %s", cfg.AssetsDir)
	log.Printf("Sync endpoint: http://%s/vst/sync", cfg.ListenAddr)
	log.Fatal(server.ListenAndServe())
}

func loadConfig() (serverConfig, error) {
	exePath, err := os.Executable()
	if err != nil {
		return serverConfig{}, fmt.Errorf("resolve executable path: %w", err)
	}

	assetsDir := filepath.Join(filepath.Dir(exePath), "assets")
	if st, statErr := os.Stat(assetsDir); statErr != nil || !st.IsDir() {
		assetsDir = filepath.Join(".", "assets")
	}

	listenAddr := envOrDefault("EVERGREEN_LISTEN_ADDR", defaultListenAddr)
	if port := strings.TrimSpace(os.Getenv("PORT")); port != "" {
		if _, err := strconv.Atoi(port); err == nil {
			listenAddr = fmt.Sprintf("0.0.0.0:%s", port)
		}
	}

	cfg := serverConfig{
		ListenAddr:        listenAddr,
		PublicBaseURL:     strings.TrimSuffix(envOrDefault("EVERGREEN_PUBLIC_BASE_URL", defaultPublicBaseURL), "/"),
		Version:           envOrDefault("EVERGREEN_VERSION", defaultVersion),
		WasmFile:          envOrDefault("EVERGREEN_WASM_FILE", defaultWasmFile),
		IconsFile:         envOrDefault("EVERGREEN_ICONS_FILE", defaultIconsFile),
		EffectsFile:       envOrDefault("EVERGREEN_EFFECTS_FILE", defaultEffectsFile),
		WebUIURL:          envOrDefault("EVERGREEN_WEB_UI_URL", defaultWebUIURL),
		AssetsDir:         envOrDefault("EVERGREEN_ASSETS_DIR", assetsDir),
		SignatureOverride: strings.TrimSpace(os.Getenv("EVERGREEN_WASM_SIGNATURE_B64")),
		SignatureFilePath: envOrDefault("EVERGREEN_SIGNATURE_FILE", defaultSignatureFile),
		RateLimitRPS:      envOrDefaultFloat("EVERGREEN_RATE_LIMIT_RPS", defaultRateLimitRPS),
		RateLimitBurst:    envOrDefaultFloat("EVERGREEN_RATE_LIMIT_BURST", defaultRateLimitBurst),
		RateLimitTTL:      envOrDefaultDuration("EVERGREEN_RATE_LIMIT_TTL", defaultRateLimitTTL),
		CORSOrigins:       parseCORSOrigins(envOrDefault("EVERGREEN_CORS_ORIGINS", defaultCORSOrigins)),
	}

	if !filepath.IsAbs(cfg.SignatureFilePath) {
		cfg.SignatureFilePath = filepath.Join(cfg.AssetsDir, cfg.SignatureFilePath)
	}

	if cfg.ListenAddr == "" {
		return serverConfig{}, errors.New("EVERGREEN_LISTEN_ADDR must not be empty")
	}
	if cfg.PublicBaseURL == "" {
		return serverConfig{}, errors.New("EVERGREEN_PUBLIC_BASE_URL must not be empty")
	}
	return cfg, nil
}

func (cfg serverConfig) resolveSignatureB64() (string, error) {
	if cfg.SignatureOverride != "" {
		return cfg.SignatureOverride, nil
	}

	bytes, err := os.ReadFile(cfg.SignatureFilePath)
	if err != nil {
		return "", fmt.Errorf("read signature file '%s': %w", cfg.SignatureFilePath, err)
	}

	return strings.TrimSpace(string(bytes)), nil
}

func envOrDefault(name, fallback string) string {
	if value, ok := os.LookupEnv(name); ok {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			return trimmed
		}
	}
	return fallback
}

func envOrDefaultFloat(name string, fallback float64) float64 {
	if value, ok := os.LookupEnv(name); ok {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			parsed, err := strconv.ParseFloat(trimmed, 64)
			if err == nil && parsed > 0 {
				return parsed
			}
		}
	}
	return fallback
}

func envOrDefaultDuration(name string, fallback time.Duration) time.Duration {
	if value, ok := os.LookupEnv(name); ok {
		trimmed := strings.TrimSpace(value)
		if trimmed != "" {
			parsed, err := time.ParseDuration(trimmed)
			if err == nil && parsed > 0 {
				return parsed
			}
		}
	}
	return fallback
}

func normalizeOrigin(origin string) string {
	trimmed := strings.TrimSpace(origin)
	return strings.TrimRight(trimmed, "/")
}

func parseCORSOrigins(value string) []string {
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return nil
	}

	parts := strings.Split(trimmed, ",")
	out := make([]string, 0, len(parts))
	for _, part := range parts {
		normalized := normalizeOrigin(part)
		if normalized == "" {
			continue
		}
		out = append(out, normalized)
	}
	return out
}

func isOriginAllowed(origin string, allowed []string) bool {
	parsed, err := url.Parse(origin)
	if err != nil {
		return false
	}
	scheme := strings.ToLower(parsed.Scheme)
	host := strings.ToLower(parsed.Hostname())
	if scheme == "" || host == "" {
		return false
	}

	for _, entry := range allowed {
		entry = strings.TrimSpace(entry)
		if entry == "" {
			continue
		}
		if entry == "*" {
			return true
		}
		if origin == entry {
			return true
		}
		if strings.HasPrefix(entry, "http://*.") || strings.HasPrefix(entry, "https://*.") {
			entryURL, err := url.Parse(entry)
			if err != nil {
				continue
			}
			entryScheme := strings.ToLower(entryURL.Scheme)
			entryHost := strings.ToLower(entryURL.Hostname())
			if entryScheme != "" && entryScheme != scheme {
				continue
			}
			if strings.HasPrefix(entryHost, "*.") {
				suffix := strings.TrimPrefix(entryHost, "*.")
				if host == suffix || strings.HasSuffix(host, "."+suffix) {
					return true
				}
			}
		}
		if strings.HasPrefix(entry, "*.") {
			suffix := strings.ToLower(strings.TrimPrefix(entry, "*."))
			if host == suffix || strings.HasSuffix(host, "."+suffix) {
				return true
			}
		}
	}
	return false
}

func setCORSHeaders(w http.ResponseWriter, r *http.Request, cfg serverConfig) {
	origin := normalizeOrigin(r.Header.Get("Origin"))
	allowedOrigin := ""
	if len(cfg.CORSOrigins) == 0 {
		allowedOrigin = "*"
	} else if origin != "" && isOriginAllowed(origin, cfg.CORSOrigins) {
		allowedOrigin = origin
	}

	if allowedOrigin != "" {
		w.Header().Set("Access-Control-Allow-Origin", allowedOrigin)
		if allowedOrigin != "*" {
			w.Header().Set("Access-Control-Allow-Credentials", "true")
			w.Header().Add("Vary", "Origin")
		}
	}

	w.Header().Set("Access-Control-Allow-Methods", "GET, OPTIONS")
	w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
}

func setSecurityHeaders(w http.ResponseWriter) {
	w.Header().Set("X-Content-Type-Options", "nosniff")
	w.Header().Set("Referrer-Policy", "no-referrer")
	w.Header().Set("X-Frame-Options", "DENY")
}

func withCORS(next http.Handler, cfg serverConfig) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		setCORSHeaders(w, r, cfg)
		setSecurityHeaders(w)
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

type tokenBucket struct {
	tokens   float64
	last     time.Time
	lastSeen time.Time
}

type rateLimiter struct {
	mu     sync.Mutex
	rps    float64
	burst  float64
	ttl    time.Duration
	buckets map[string]*tokenBucket
}

func newRateLimiter(rps, burst float64, ttl time.Duration) *rateLimiter {
	if rps <= 0 {
		rps = defaultRateLimitRPS
	}
	if burst <= 0 {
		burst = defaultRateLimitBurst
	}
	if ttl <= 0 {
		ttl = defaultRateLimitTTL
	}
	return &rateLimiter{
		rps:     rps,
		burst:   burst,
		ttl:     ttl,
		buckets: make(map[string]*tokenBucket),
	}
}

func (rl *rateLimiter) allow(key string) bool {
	now := time.Now()
	rl.mu.Lock()
	defer rl.mu.Unlock()

	bucket, ok := rl.buckets[key]
	if !ok {
		rl.buckets[key] = &tokenBucket{
			tokens:   rl.burst - 1,
			last:     now,
			lastSeen: now,
		}
		return true
	}

	elapsed := now.Sub(bucket.last).Seconds()
	bucket.tokens = minFloat(rl.burst, bucket.tokens+(elapsed*rl.rps))
	bucket.last = now
	bucket.lastSeen = now

	if bucket.tokens < 1 {
		return false
	}
	bucket.tokens -= 1

	rl.cleanup(now)
	return true
}

func (rl *rateLimiter) cleanup(now time.Time) {
	for key, bucket := range rl.buckets {
		if now.Sub(bucket.lastSeen) > rl.ttl {
			delete(rl.buckets, key)
		}
	}
}

func minFloat(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

func withRateLimit(next http.Handler, limiter *rateLimiter) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if limiter == nil {
			next.ServeHTTP(w, r)
			return
		}

		ip := clientIP(r)
		if !limiter.allow(ip) {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusTooManyRequests)
			_, _ = w.Write([]byte(`{"error":"rate limit exceeded"}`))
			return
		}
		next.ServeHTTP(w, r)
	})
}

func clientIP(r *http.Request) string {
	if forwarded := r.Header.Get("X-Forwarded-For"); forwarded != "" {
		parts := strings.Split(forwarded, ",")
		if len(parts) > 0 {
			candidate := strings.TrimSpace(parts[0])
			if candidate != "" {
				return candidate
			}
		}
	}
	return strings.Split(r.RemoteAddr, ":")[0]
}
