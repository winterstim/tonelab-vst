import React, { useState, useEffect } from 'react';
import './Knob.css';

export default function Knob({ size = 106, value = 0.5, label = "", onChange }) {






    const [isDragging, setIsDragging] = useState(false);




    const degrees = (value * 270) - 135;

    const handleMouseDown = (e) => {

        e.stopPropagation();
        e.preventDefault();
        setIsDragging(true);

    };

    useEffect(() => {
        const handleMouseMove = (e) => {
            if (!isDragging) return;






            const deltaX = e.movementX;
            const deltaY = -e.movementY;

            const sensitivity = 0.005;
            const delta = (deltaX + deltaY) * sensitivity;






            let newValue = value + delta;


            if (newValue < 0) newValue = 0;
            if (newValue > 1) newValue = 1;

            if (onChange) onChange(newValue);
        };

        const handleMouseUp = () => {
            if (isDragging) setIsDragging(false);
        };

        if (isDragging) {
            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
            document.body.style.cursor = 'ns-resize';
        }

        return () => {
            window.removeEventListener('mousemove', handleMouseMove);
            window.removeEventListener('mouseup', handleMouseUp);
            document.body.style.cursor = 'default';
        };
    }, [isDragging, value, onChange]);

    return (
        <div
            className="knob-container nodrag nopan"
            style={{ width: size, height: size, position: 'relative' }}
            onMouseDown={handleMouseDown}
        >
            <div
                className="liquid-glass-knob-surface"
                style={{
                    position: 'absolute',
                    left: '15.52%',
                    top: '4.78%',
                    width: '66.04%',
                    aspectRatio: '1 / 1',
                    borderRadius: '50%',
                    boxSizing: 'border-box',
                    pointerEvents: 'none'
                }}
            />
            <svg width="100%" height="100%" viewBox="0 0 106 115" fill="none" style={{ position: 'relative', zIndex: 1 }}>
                { }
                { }
                <g>
                    <text
                        x="53"
                        y="98"
                        textAnchor="middle"
                        fill="white"
                        fillOpacity="0.7"
                        style={{
                            fontSize: '9px',
                            fontFamily: 'Inter, sans-serif',
                            letterSpacing: '0.08em',
                            textTransform: 'uppercase',

                            fontWeight: 400,
                            filter: 'drop-shadow(0px 1px 2px rgba(0,0,0,0.5))',
                            textRendering: 'geometricPrecision'
                        }}
                    >
                        {label}
                    </text>
                </g>

                { }

                { }
                { }

                { }
                <g transform={`rotate(${degrees} 51.4551 40.5)`}>
                    { }
                    <path d="M51.4551 11.5V35" stroke="white" strokeWidth="3" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
                </g>
            </svg>
        </div>
    );
}
