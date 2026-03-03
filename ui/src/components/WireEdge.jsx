import React from 'react';

const buildPath = (x1, y1, x2, y2) => {
    return `M ${x1} ${y1} L ${x2} ${y2}`;
};

const edgeToCenter = (x, y, position, radius) => {
    if (!radius) return { x, y };
    switch (position) {
        case 'left':
            return { x: x + radius, y };
        case 'right':
            return { x: x - radius, y };
        case 'top':
            return { x, y: y + radius };
        case 'bottom':
            return { x, y: y - radius };
        default:
            return { x, y };
    }
};

export default function WireEdge({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, data }) {
    const strokeWidth = data?.strokeWidth ?? 2;
    const sourceRadius = data?.sourceRadius ?? 0;
    const targetRadius = data?.targetRadius ?? 0;
    const source = edgeToCenter(sourceX, sourceY, sourcePosition, sourceRadius);
    const target = edgeToCenter(targetX, targetY, targetPosition, targetRadius);
    const pathData = buildPath(source.x, source.y, target.x, target.y);

    return (
        <path
            d={pathData}
            stroke="#888"
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            fill="none"
            className="wire-path"
        />
    );
}
