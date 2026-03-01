import React, { useState, useEffect, useRef } from 'react';
import Knob from './Knob';
import { EFFECTS_METADATA } from '../config/effects';
import {
    COL_GAP,
    COL_WIDTH,
    KNOB_GAP,
    KNOB_SIZE,
    PADDING_V,
    getCardMetrics
} from '../utils/layout';
import './Card.css';

function Card({
    id,
    x,
    y,
    selected,
    onMove,
    externalDrag = false,
    showPorts = true,
    ...props
}) {

    const [isDragging, setIsDragging] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

    const cardRef = useRef(null);


    useEffect(() => {
        return () => {
            setIsDragging(false);
        };
    }, []);


    const handleMouseDown = (e) => {
        if (externalDrag) return;

        if (e.target.closest('.knob-container')) return;

        e.stopPropagation();


        if (props.onSelect) props.onSelect(id, e);

        setIsDragging(true);
        setDragStart({ x: e.clientX, y: e.clientY });
    };

    useEffect(() => {
        const handleMouseMove = (e) => {
            if (isDragging) {
                const dx = (e.clientX - dragStart.x);
                const dy = (e.clientY - dragStart.y);

                onMove(id, dx, dy);
                setDragStart({ x: e.clientX, y: e.clientY });
            }
        };

        const handleMouseUp = () => {
            setIsDragging(false);
        };

        if (isDragging) {
            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
        }
        return () => {
            window.removeEventListener('mousemove', handleMouseMove);
            window.removeEventListener('mouseup', handleMouseUp);
        };
    }, [isDragging, dragStart, id, onMove]);



    const effectMeta = EFFECTS_METADATA[props.type];
    const paramsList = effectMeta ? Object.entries(effectMeta.params) : [];
    const paramCount = paramsList.length;

    const {
        columns,
        rows,
        placements,
        bodyWidth,
        bodyHeight,
        svgWidth,
        svgHeight
    } = getCardMetrics(paramCount);

    const width = svgWidth;
    const height = svgHeight;


    if (props.layer === 'back') {
        return (
            <div
                className={`card-node-back`}
                style={{
                    transform: `translate(${x}px, ${y}px)`,
                    width: width,
                    height: height,
                    position: 'absolute',
                    top: 0, left: 0, pointerEvents: 'none', zIndex: 0
                }}
            >
                <div style={{ width: svgWidth, height: svgHeight, position: 'relative' }}>
                    <svg width={svgWidth} height={svgHeight} viewBox={`0 0 ${svgWidth} ${svgHeight}`} fill="none" style={{ overflow: 'visible' }}>
                        <defs>
                            <linearGradient id={`ear_l_${id}`} x1="4" y1="158" x2="29" y2="183" gradientUnits="userSpaceOnUse">
                                <stop stopColor="white" stopOpacity="0.6" />
                                <stop offset="1" stopColor="white" stopOpacity="0.1" />
                            </linearGradient>
                            <linearGradient id={`ear_r_${id}`} x1={50 + bodyWidth + 21} y1="158" x2={50 + bodyWidth + 46} y2="183" gradientUnits="userSpaceOnUse">
                                <stop stopColor="white" stopOpacity="0.6" />
                                <stop offset="1" stopColor="white" stopOpacity="0.1" />
                            </linearGradient>
                        </defs>
                        {}
                        <g>
                            <rect x="4" y="158" width="25" height="25" rx="12.5" fill="white" fillOpacity="0.05" shapeRendering="geometricPrecision" />
                            <rect x="4.5" y="158.5" width="24" height="24" rx="12" stroke={`url(#ear_l_${id})`} strokeOpacity="0.25" shapeRendering="geometricPrecision" />
                        </g>

                        {}
                        <g>
                            <rect x={50 + bodyWidth + 21} y="158" width="25" height="25" rx="12.5" fill="white" fillOpacity="0.05" shapeRendering="geometricPrecision" />
                            <rect x={50 + bodyWidth + 21 + 0.5} y="158.5" width="24" height="24" rx="12" stroke={`url(#ear_r_${id})`} strokeOpacity="0.25" shapeRendering="geometricPrecision" />
                        </g>
                    </svg>
                </div>
            </div>
        );
    }

    return (
        <div
            className={`card-node ${selected ? 'selected' : ''}`}
            ref={cardRef}
            style={{
                transform: `translate(${x}px, ${y}px)`,
                width: width,
                height: height,
                zIndex: isDragging || selected ? 10 : 1,
            }}
            onMouseDown={handleMouseDown}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => setIsHovered(false)}
        >
            <div
                className="card-content"
                style={{
                    width: svgWidth,
                    height: svgHeight,
                    position: 'relative'
                }}
            >
                {}
                {props.layer !== 'back' && (
                    <div style={{
                        position: 'absolute',
                        top: '-50px',
                        left: 50,
                        width: bodyWidth,
                        textAlign: 'center',
                        color: (selected || isHovered) ? 'rgba(255, 255, 255, 1)' : 'rgba(255, 255, 255, 0.7)',
                        fontSize: '24px',
                        fontWeight: '500',
                        letterSpacing: '0.5px',
                        pointerEvents: 'auto',
                        cursor: 'grab',
                        userSelect: 'none',
                        fontFamily: 'sans-serif',
                        textShadow: selected ? '0 0 15px rgba(255, 255, 255, 0.5)' : '0 2px 10px rgba(0,0,0,0.5)',
                        transition: 'color 0.2s, text-shadow 0.2s'
                    }}>
                        {props.type}
                    </div>
                )}

                {}
                {}

                {}
                <div className="liquid-glass-surface liquid-glass-node-surface" style={{
                    position: 'absolute',
                    top: 0,
                    left: 50,
                    width: bodyWidth,
                    height: bodyHeight,
                    borderRadius: 24,
                    boxSizing: 'border-box',
                    pointerEvents: 'none'
                }} />

                {}
                {}
                {showPorts && (
                    <div className="port port-in nodrag nopan" style={{
                    left: -21, top: 133, width: 75, height: 75, borderRadius: '50%', position: 'absolute',
                    cursor: 'crosshair', zIndex: 100, pointerEvents: 'auto',
                    background: 'transparent', border: 'none', boxShadow: 'none'
                }}
                    onMouseDown={(e) => { e.stopPropagation(); e.preventDefault();  }}
                    onMouseUp={(e) => { e.stopPropagation(); if (props.onPortMouseDown) props.onPortMouseDown(id, 'in', e, 'up'); }}
                />
                )}
                {showPorts && (
                    <div className="port port-out nodrag nopan" style={{
                    left: 50 + bodyWidth - 4, top: 133, width: 75, height: 75, borderRadius: '50%', position: 'absolute',
                    cursor: 'crosshair', zIndex: 100, pointerEvents: 'auto',
                    background: 'transparent', border: 'none', boxShadow: 'none'
                }}
                    onMouseDown={(e) => { e.stopPropagation(); e.preventDefault(); if (props.onPortMouseDown) props.onPortMouseDown(id, 'out', e, 'down'); }}
                />
                )}

                {}
                <div className="card-controls" style={{
                    position: 'absolute',
                    top: 0,
                    left: 50,
                    width: bodyWidth,
                    height: bodyHeight,
                    pointerEvents: 'none',
                    display: 'grid',
                    gridTemplateColumns: `repeat(${columns}, ${COL_WIDTH}px)`,
                    gridTemplateRows: `repeat(${rows}, ${KNOB_SIZE}px)`,
                    columnGap: `${COL_GAP}px`,
                    rowGap: `${KNOB_GAP}px`,
                    placeContent: 'center',
                    justifyItems: 'center',
                    alignItems: 'center',
                    padding: `${PADDING_V}px 0`
                }}>
                    {paramsList.length > 0 ? (
                        paramsList.map(([key, config], index) => {
                            const placement = placements[index] || { row: 1, col: 1 };
                            const gridStyle = {
                                gridColumn: `${placement.col} / span ${placement.colSpan || 1}`,
                                gridRow: `${placement.row} / span ${placement.rowSpan || 1}`,
                                justifySelf: 'center',
                                alignSelf: 'center',
                                pointerEvents: 'auto'
                            };

                            const realValue = props.params[key] ?? config.default;
                            const range = config.max - config.min;
                            const normalizedValue = (realValue - config.min) / range;

                            return (
                                <div key={key} style={gridStyle}>
                                    <Knob
                                        size={KNOB_SIZE}
                                        value={normalizedValue}
                                        label={config.label}
                                        onChange={(normVal) => {
                                            let newVal = (normVal * range) + config.min;
                                            if (config.step) newVal = Math.round(newVal / config.step) * config.step;
                                            if (newVal < config.min) newVal = config.min;
                                            if (newVal > config.max) newVal = config.max;
                                            props.onParamChange && props.onParamChange(id, key, newVal);
                                        }}
                                    />
                                </div>
                            );
                        })
                    ) : (
                        <div style={{ color: 'red' }}>Unknown Type</div>
                    )}
                </div>
            </div>

        </div>
    );
}


export default Card;
