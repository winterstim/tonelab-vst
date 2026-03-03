import React from 'react';
import { ReactFlow, SelectionMode, Background, BackgroundVariant } from '@xyflow/react';
import FlowCardNode from './FlowCardNode';
import WireEdge from './WireEdge';
import ConnectionLine from './ConnectionLine';
import { SVGDefs } from './SVGDefs';

const nodeTypes = { card: FlowCardNode };
const edgeTypes = { wire: WireEdge };
const GRID_SIZE = 210;
const GRID_STROKE = 'rgba(255, 255, 255, 0.025)';

export default function Dashboard({
    flowNodes,
    flowEdges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    onNodeClick,
    onNodeContextMenu,
    isValidConnection,
    onSelectionStart,
    onSelectionEnd,
    onViewportChange,
}) {
    return (
        <div
            style={{
                width: '100%',
                height: '100%',
                position: 'fixed',
                top: 0,
                left: 0,
                backgroundColor: '#0a0a0a',
                border: '1px solid var(--dashboard-border)',
                overflow: 'hidden',

                userSelect: 'none',
                WebkitUserSelect: 'none'
            }}
        >
            <ReactFlow
                nodes={flowNodes}
                edges={flowEdges}
                nodeTypes={nodeTypes}
                edgeTypes={edgeTypes}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onNodeClick={onNodeClick}
                onNodeContextMenu={onNodeContextMenu}
                isValidConnection={isValidConnection}
                onMove={(event, viewport) => {
                    if (onViewportChange) onViewportChange(viewport);
                }}
                onSelectionStart={onSelectionStart}
                onSelectionEnd={onSelectionEnd}
                connectionMode="strict"
                connectionLineComponent={ConnectionLine}
                noDragClassName="nodrag"
                noPanClassName="nopan"
                nodeOrigin={[0, 0]}
                elevateNodesOnSelect={false}
                selectionKeyCode="Shift"
                multiSelectionKeyCode={null}
                selectionMode={SelectionMode.Partial}
                connectOnClick={false}
                zoomOnDoubleClick={false}
                panOnDrag={true}
                preventScrolling={true}
                minZoom={0.1}
                maxZoom={5}
                translateExtent={[[-50000, -50000], [50000, 50000]]}
                proOptions={{ hideAttribution: true }}
                nodesFocusable={true}
                edgesFocusable={false}
                style={{
                    backgroundColor: '#0a0a0a',
                    width: '100%',
                    height: '100%',
                    zIndex: 1,
                }}
            >
                <Background
                    color={GRID_STROKE}
                    gap={GRID_SIZE}
                    size={1}
                    variant={BackgroundVariant.Lines}
                />
                <SVGDefs />
            </ReactFlow>
        </div>
    );
}
