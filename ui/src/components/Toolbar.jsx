import React, { useEffect, useRef, useState } from 'react';
import {
    LogoIcon,
    LogoIconWhite,
    WaveIcon,
    TrapezoidIcon,
    ArrowsIcon,
    SquigglyIcon,
    ClockIcon,
    DotsIcon
} from './Icons';
import { generateChainFromPrompt } from '../services/tonelabApi';
import { buildWebUrl } from '../config/runtime';
import { openExternalUrl } from '../utils/externalNavigation';
import './Toolbar.css';

import { Tooltip } from './Tooltip';

export default function Toolbar({ onSpawn, onLoadChain, isHovered }) {
    const [hoveredItem, setHoveredItem] = useState(null);
    const [isOptionsOpen, setIsOptionsOpen] = useState(false);
    const [isInputMode, setIsInputMode] = useState(false);
    const [inputValue, setInputValue] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [statusText, setStatusText] = useState('');
    const [errorText, setErrorText] = useState('');
    const submitAbortRef = useRef(null);

    const options = [
        { id: 'btn1', type: 'Equalizer', label: 'Equalizer', Icon: WaveIcon },
        { id: 'btn2', type: 'Overdrive', label: 'Overdrive', Icon: TrapezoidIcon },
        { id: 'btn3', type: 'NoiseGate', label: 'Noise Gate', Icon: ArrowsIcon },
        { id: 'btn4', type: 'Reverb', label: 'Reverb', Icon: SquigglyIcon },
        { id: 'btn5', type: 'Delay', label: 'Delay', Icon: ClockIcon },
    ];

    useEffect(() => {
        const handleClickOutside = () => setIsOptionsOpen(false);
        document.addEventListener('click', handleClickOutside);
        return () => {
            document.removeEventListener('click', handleClickOutside);
            if (submitAbortRef.current) {
                submitAbortRef.current.abort();
            }
        };
    }, []);

    useEffect(() => {
        if (!statusText || isSubmitting) return undefined;
        const timeout = setTimeout(() => setStatusText(''), 4000);
        return () => clearTimeout(timeout);
    }, [statusText, isSubmitting]);

    const cancelInFlightSubmit = () => {
        if (!submitAbortRef.current) return;
        submitAbortRef.current.abort();
        submitAbortRef.current = null;
    };

    const closeInputMode = () => {
        cancelInFlightSubmit();
        setIsSubmitting(false);
        setStatusText('');
        setErrorText('');
        setIsInputMode(false);
    };

    const handleSubmit = async () => {
        const prompt = inputValue.trim();
        if (!prompt || isSubmitting) return;

        cancelInFlightSubmit();
        const controller = new AbortController();
        submitAbortRef.current = controller;

        setIsSubmitting(true);
        setErrorText('');
        setStatusText('Preparing request...');

        try {
            const chain = await generateChainFromPrompt(prompt, {
                signal: controller.signal,
                onStatus: (message) => {
                    if (message) setStatusText(message);
                }
            });

            if (onLoadChain) onLoadChain(chain);
            setInputValue('');
            setStatusText(
                chain.length > 0
                    ? `Loaded ${chain.length} effect${chain.length === 1 ? '' : 's'}.`
                    : 'AI returned an empty chain.'
            );
        } catch (error) {
            if (error?.name === 'AbortError') return;
            setErrorText(error?.message || 'Failed to generate chain.');
        } finally {
            if (submitAbortRef.current === controller) {
                submitAbortRef.current = null;
            }
            setIsSubmitting(false);
        }
    };

    const canSubmit = !isSubmitting && inputValue.trim().length > 0;
    const inputPlaceholder =
        errorText ||
        statusText ||
        (isSubmitting ? 'Processing...' : 'Type command...');

    return (
        <div className={`toolbar-wrapper ${isHovered ? 'force-hover' : ''}`}>
            <div className="toolbar-underglow" />

            <div className="toolbar liquid-glass-surface liquid-glass-toolbar-surface" onClick={(e) => e.stopPropagation()} onMouseDown={(e) => e.stopPropagation()}>

                <div
                    className="toolbar-item logo"
                    onClick={() => {
                        if (isInputMode) {
                            closeInputMode();
                            return;
                        }
                        setIsInputMode(true);
                        setErrorText('');
                        setStatusText('');
                    }}
                    onMouseEnter={() => setHoveredItem('logo')}
                    onMouseLeave={() => setHoveredItem(null)}
                    style={{ cursor: 'pointer', position: 'relative' }}
                >
                    <div className="logo-wrapper">
                        <LogoIcon className="logo-gradient" />
                        <LogoIconWhite className="logo-grey" />
                    </div>
                    {hoveredItem === 'logo' && <Tooltip text="Tonelab AI" />}
                </div>

                <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'flex-start' }}>

                    {!isInputMode ? (
                        <>
                            {options.map((item) => (
                                <div
                                    key={item.id}
                                    className="toolbar-item option"
                                    style={{ position: 'relative' }}
                                    onMouseDown={(e) => {
                                        e.stopPropagation();
                                        onSpawn(item.type);
                                    }}
                                    onMouseEnter={() => setHoveredItem(item.id)}
                                    onMouseLeave={() => setHoveredItem(null)}
                                >
                                    <item.Icon />
                                    {hoveredItem === item.id && <Tooltip text={item.label} />}
                                </div>
                            ))}

                            <div
                                className="toolbar-item dots"
                                style={{ position: 'relative' }}
                                onClick={(e) => {
                                    e.stopPropagation();
                                    setIsOptionsOpen(!isOptionsOpen);
                                    setHoveredItem(null); // Hide tooltip when opening
                                }}
                                onMouseEnter={() => !isOptionsOpen && setHoveredItem('dots')}
                                onMouseLeave={() => setHoveredItem(null)}
                            >
                                <DotsIcon />
                                {hoveredItem === 'dots' && !isOptionsOpen && <Tooltip text="Options" />}

                                {isOptionsOpen && (
                                    <div className="menu-popup" onClick={(e) => e.stopPropagation()}>
                                        <button
                                            className="menu-button"
                                            onClick={() => {
                                                openExternalUrl(buildWebUrl('/docs'));
                                                setIsOptionsOpen(false);
                                            }}
                                        >
                                            Documentation
                                        </button>
                                        <button
                                            className="menu-button"
                                            onClick={() => {
                                                openExternalUrl(buildWebUrl('/user/cabinet'));
                                                setIsOptionsOpen(false);
                                            }}
                                        >
                                            Personal Cabinet
                                        </button>
                                    </div>
                                )}
                            </div>
                        </>
                    ) : (
                        <div style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'center', paddingLeft: '6px', paddingRight: '0px', position: 'relative' }}>
                            <input
                                id="toolbar-input"
                                type="text"
                                placeholder={inputPlaceholder}
                                autoFocus
                                value={inputValue}
                                style={{
                                    flex: 1,
                                    background: 'transparent',
                                    border: 'none',
                                    color: errorText ? 'rgba(255, 150, 150, 0.95)' : 'rgba(255, 255, 255, 0.9)',
                                    fontSize: '14px',
                                    fontFamily: 'sans-serif',
                                    fontWeight: '500',
                                    outline: 'none',
                                    letterSpacing: '0.5px',
                                    lineHeight: '1',
                                    transform: 'translateY(1px)'
                                }}
                                onChange={(e) => {
                                    setInputValue(e.target.value);
                                    if (errorText) setErrorText('');
                                    if (!isSubmitting && statusText) setStatusText('');
                                }}
                                onKeyDown={(e) => {
                                    if (e.key === 'Enter') {
                                        e.preventDefault();
                                        handleSubmit();
                                    }
                                    if (e.key === 'Escape') {
                                        closeInputMode();
                                    }
                                }}
                            />
                            <div
                                style={{
                                    width: '26px',
                                    height: '26px',
                                    borderRadius: '50%',
                                    background: 'white',
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'center',
                                    cursor: canSubmit ? 'pointer' : 'not-allowed',
                                    marginLeft: '12px',
                                    boxShadow: '0 2px 4px rgba(0,0,0,0.2)',
                                    transition: 'transform 0.1s, opacity 0.2s',
                                    marginRight: '-2px',
                                    opacity: canSubmit ? 1 : 0.55
                                }}
                                onClick={() => {
                                    if (!canSubmit) return;
                                    handleSubmit();
                                }}
                                onMouseEnter={(e) => {
                                    if (!canSubmit) return;
                                    e.currentTarget.style.transform = 'scale(1.1)';
                                }}
                                onMouseLeave={(e) => { e.currentTarget.style.transform = 'scale(1)'; }}
                            >
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="black" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                                    <path d="M12 19V5" />
                                    <path d="M5 12l7-7 7 7" />
                                </svg>
                            </div>
                            {(statusText || errorText) && (
                                <div
                                    style={{
                                        position: 'absolute',
                                        left: '4px',
                                        right: '44px',
                                        bottom: '-16px',
                                        fontSize: '10px',
                                        color: errorText ? 'rgba(255, 150, 150, 0.95)' : 'rgba(255, 255, 255, 0.7)',
                                        lineHeight: 1.2,
                                        whiteSpace: 'nowrap',
                                        overflow: 'hidden',
                                        textOverflow: 'ellipsis',
                                        pointerEvents: 'none',
                                        zIndex: 20
                                    }}
                                >
                                    {errorText || statusText}
                                </div>
                            )}
                        </div>
                    )}
                </div>
            </div>
        </div >
    );
}
