import React, { useEffect, useRef } from 'react';

const ArrowSVG = ({ rotate, size }) => (
    <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        style={{
            transform: `rotate(${rotate}deg)`,
            pointerEvents: 'none',
            filter: 'drop-shadow(0 2px 4px rgba(0,0,0,0.5))'
        }}
    >
        <path
            d="M7 17 L7 7 L17 7"
            stroke="currentColor"
            strokeWidth={Math.max(1.6, size / 20)}
            strokeLinecap="round"
            strokeLinejoin="round"
        />
    </svg>
);

export default function GroupScaleOverlay({ bounds, view, onScaleStart, onScale, onScaleEnd }) {
    const draggingRef = useRef(false);
    const viewRef = useRef(view);

    useEffect(() => {
        viewRef.current = view;
    }, [view]);

    if (!bounds) return null;

    const { minX, minY, maxX, maxY } = bounds;
    const width = maxX - minX;
    const height = maxY - minY;

    const handleSize = Math.max(28, Math.min(56, Math.min(width, height) * 0.06));

    const handleStyle = (left, top) => ({
        position: 'absolute',
        left,
        top,
        width: handleSize,
        height: handleSize,
        color: 'rgba(255, 255, 255, 0.6)',
        cursor: 'nwse-resize',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        pointerEvents: 'auto'
    });

    const startDrag = (event) => {
        event.stopPropagation();
        event.preventDefault();
        draggingRef.current = true;
        onScaleStart();

        const handleMouseMove = (moveEvent) => {
            if (!draggingRef.current) return;
            const currentView = viewRef.current;
            const worldX = (moveEvent.clientX - currentView.x) / currentView.zoom;
            const worldY = (moveEvent.clientY - currentView.y) / currentView.zoom;
            onScale(worldX, worldY);
        };

        const handleMouseUp = () => {
            if (!draggingRef.current) return;
            draggingRef.current = false;
            onScaleEnd();
            window.removeEventListener('mousemove', handleMouseMove);
            window.removeEventListener('mouseup', handleMouseUp);
        };

        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    };

    return (
        <div
            style={{
                position: 'absolute',
                inset: 0,
                zIndex: 4,
                pointerEvents: 'none'
            }}
        >
            <div
                style={{
                    position: 'absolute',
                    transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})`,
                    transformOrigin: '0 0',
                    pointerEvents: 'none'
                }}
            >
                <div
                    style={handleStyle(minX - handleSize / 2, minY - handleSize / 2)}
                    onMouseDown={startDrag}
                >
                    <ArrowSVG rotate={0} size={handleSize} />
                </div>
                <div
                    style={{ ...handleStyle(maxX - handleSize / 2, minY - handleSize / 2), cursor: 'nesw-resize' }}
                    onMouseDown={startDrag}
                >
                    <ArrowSVG rotate={90} size={handleSize} />
                </div>
                <div
                    style={{ ...handleStyle(minX - handleSize / 2, maxY - handleSize / 2), cursor: 'nesw-resize' }}
                    onMouseDown={startDrag}
                >
                    <ArrowSVG rotate={270} size={handleSize} />
                </div>
                <div
                    style={handleStyle(maxX - handleSize / 2, maxY - handleSize / 2)}
                    onMouseDown={startDrag}
                >
                    <ArrowSVG rotate={180} size={handleSize} />
                </div>
            </div>
        </div>
    );
}
