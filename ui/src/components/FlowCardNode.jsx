import React, { useEffect, useMemo } from 'react';
import { Handle, Position, useUpdateNodeInternals } from '@xyflow/react';
import Card from './Card';
import { EFFECTS_METADATA } from '../config/effects';
import { getCardMetrics } from '../utils/layout';

const BASE_INPUT = { left: -21, top: 133, size: 75 };

export default function FlowCardNode({ id, data, selected }) {
    const updateNodeInternals = useUpdateNodeInternals();
    const { node, onParamChange, forceSelected } = data;

    const paramCount = useMemo(() => {
        const effectMeta = EFFECTS_METADATA[node.type];
        return effectMeta ? Object.keys(effectMeta.params).length : 0;
    }, [node.type]);

    const { bodyWidth, svgWidth, svgHeight } = useMemo(() => getCardMetrics(paramCount), [paramCount]);

    const scaledWidth = svgWidth;
    const scaledHeight = svgHeight;

    const portCenterY = BASE_INPUT.top + (BASE_INPUT.size / 2);
    const inputCenterX = BASE_INPUT.left + (BASE_INPUT.size / 2);
    const outputCenterX = 50 + bodyWidth - 4 + (BASE_INPUT.size / 2);
    const handleSize = 25;
    const handleTop = portCenterY - (handleSize / 2);

    useEffect(() => {
        updateNodeInternals(id);
    }, [id, node.type, updateNodeInternals]);

    return (
        <div style={{ width: scaledWidth, height: scaledHeight, position: 'relative' }}>
            <Card
                id={id}
                x={0}
                y={0}
                type={node.type}
                params={node.params}
                layer="back"
                externalDrag={true}
                showPorts={false}
            />
            <Card
                id={id}
                x={0}
                y={0}
                type={node.type}
                params={node.params}
                layer="front"
                selected={selected || forceSelected}
                onParamChange={onParamChange}
                externalDrag={true}
                showPorts={false}
            />

            <Handle
                type="target"
                position={Position.Left}
                isConnectableStart={false}
                isConnectableEnd={true}
                className="port-handle nodrag nopan"
                style={{
                    left: inputCenterX - (handleSize / 2),
                    top: handleTop,
                    width: handleSize,
                    height: handleSize,
                    transform: 'none',
                    background: 'transparent',
                    border: 'none',
                    borderRadius: '50%',
                    zIndex: 100
                }}
            />
            <Handle
                type="source"
                position={Position.Right}
                isConnectableStart={true}
                isConnectableEnd={false}
                className="port-handle nodrag nopan"
                style={{
                    left: outputCenterX - (handleSize / 2),
                    top: handleTop,
                    width: handleSize,
                    height: handleSize,
                    transform: 'none',
                    background: 'transparent',
                    border: 'none',
                    borderRadius: '50%',
                    zIndex: 100
                }}
            />
        </div>
    );
}
