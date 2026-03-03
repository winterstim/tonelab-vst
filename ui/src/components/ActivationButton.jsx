import { Tooltip } from './Tooltip';
import React, { useState } from 'react';

const ActivationButton = ({ state, onClick, isHovered }) => {
    const [showTooltip, setShowTooltip] = useState(false);
    const size = 48;

    const renderIconPath = () => (
        state === 'start' ? (
            <path
                d="M8 6V18L18 12L8 6Z"
                fill="currentColor"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinejoin="round"
            />
        ) : (
            <g fill="currentColor">
                <rect x="6" y="5" width="4" height="14" rx="2" />
                <rect x="14" y="5" width="4" height="14" rx="2" />
            </g>
        )
    );

    return (
        <div
            className={`activation-button-wrapper liquid-glass-surface liquid-glass-toolbar-surface ${isHovered ? 'force-hover' : ''}`}
            onClick={(e) => { e.stopPropagation(); onClick(); }}
            onMouseDown={(e) => e.stopPropagation()}
            onMouseEnter={() => setShowTooltip(true)}
            onMouseLeave={() => setShowTooltip(false)}
            style={{
                width: size, height: size,
                borderRadius: '50%',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                cursor: 'pointer',
                transition: 'all 0.3s ease',
                position: 'relative',

                color: '#BFBFBF'
            }}
        >
            {/* Base icon (grey) */}
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" style={{ position: 'absolute', transition: 'all 0.3s ease' }}>
                {renderIconPath()}
            </svg>

            {/* Gradient icon (overlay, fades in on hover) */}
            <svg
                width="28" height="28" viewBox="0 0 24 24" fill="none"
                className="activation-icon-gradient"
                style={{
                    position: 'absolute',
                    overflow: 'visible',
                    willChange: 'opacity, filter',
                    transition: 'all 0.3s ease',
                    opacity: 0,
                }}
            >
                <defs>
                    <linearGradient id="activation-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stopColor="#00FFD1" />
                        <stop offset="25%" stopColor="#00BFFF" />
                        <stop offset="50%" stopColor="#005DFF" />
                        <stop offset="75%" stopColor="#4D00FF" />
                        <stop offset="100%" stopColor="#8A00FF" />
                    </linearGradient>
                </defs>
                {/* Re-use path with gradient fill reference */}
                {renderIconPath()}
            </svg>

            {showTooltip && (
                <Tooltip text={state === 'start' ? 'Play' : 'Pause'} />
            )}

            <style>{`
        /* Button hover raise effect */
        .activation-button-wrapper:hover,
        .activation-button-wrapper.force-hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(0,0,0,0.3);
            color: #fff;
        }

        /* Show gradient icon on hover */
        .activation-button-wrapper:hover .activation-icon-gradient {
            opacity: 1 !important; 
        }
        
        /* Apply gradient to the overlay icon */
        .activation-icon-gradient path,
        .activation-icon-gradient rect {
            fill: url(#activation-gradient);
            stroke: url(#activation-gradient);
            filter: drop-shadow(0 0 5px rgba(0, 180, 255, 1)) drop-shadow(0 0 15px rgba(0, 180, 255, 0.6));
        }
      `}</style>
        </div>
    );
};

export default ActivationButton;
