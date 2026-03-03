import React from 'react';

const buildPath = (x1, y1, x2, y2) => {
    return `M ${x1} ${y1} L ${x2} ${y2}`;
};

export default function ConnectionLine({ fromX, fromY, toX, toY }) {
    const pathData = buildPath(fromX, fromY, toX, toY);

    return (
        <path
            d={pathData}
            stroke="#fff"
            strokeWidth="2"
            strokeLinecap="round"
            fill="none"
            className="wire-path wire-path--active"
        />
    );
}
