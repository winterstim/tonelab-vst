import React from 'react';

export default function Wire({ x1, y1, x2, y2, active }) {
    const pathData = `M ${x1} ${y1} L ${x2} ${y2}`;

    return (
        <svg
            style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: '100%',
                pointerEvents: 'none',
                overflow: 'visible'
            }}
        >

            {}
            <path
                d={pathData}
                stroke={active ? "#fff" : "#888"}
                strokeWidth="2"
                strokeLinecap="round"
                fill="none"
                className="wire-path"
            />
        </svg>
    );
}
