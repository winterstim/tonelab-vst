function postViaWryIpc(message) {
    if (typeof window === 'undefined') return false;
    if (typeof window.ipc?.postMessage === 'function') {
        window.ipc.postMessage(message);
        return true;
    }
    return false;
}

function postViaWebkit(message) {
    if (typeof window === 'undefined') return false;
    const handler = window.webkit?.messageHandlers?.ipc;
    if (handler && typeof handler.postMessage === 'function') {
        handler.postMessage(message);
        return true;
    }
    return false;
}

function postViaWebView2(message) {
    if (typeof window === 'undefined') return false;
    if (typeof window.chrome?.webview?.postMessage === 'function') {
        window.chrome.webview.postMessage(message);
        return true;
    }
    return false;
}

export function postIpcMessage(payload) {
    const message = typeof payload === 'string' ? payload : JSON.stringify(payload);
    return (
        postViaWryIpc(message) ||
        postViaWebkit(message) ||
        postViaWebView2(message)
    );
}

export function hasIpcBridge() {
    if (typeof window === 'undefined') return false;
    return (
        typeof window.ipc?.postMessage === 'function' ||
        typeof window.webkit?.messageHandlers?.ipc?.postMessage === 'function' ||
        typeof window.chrome?.webview?.postMessage === 'function'
    );
}
