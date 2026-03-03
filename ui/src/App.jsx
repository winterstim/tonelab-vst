import React, { useState, useMemo, useCallback, useRef } from 'react';
import { applyNodeChanges } from '@xyflow/react';
import Toolbar from './components/Toolbar';
import Dashboard from './components/Dashboard';
import ActivationButton from './components/ActivationButton';
import UpdateNotice from './components/UpdateNotice';
import { useBridge } from './hooks/useBridge';
import { fetchPluginUpdateInfo, installPluginUpdate } from './services/pluginUpdateApi';

import { EFFECTS_METADATA } from './config/effects';
import { getCardMetrics } from './utils/layout';

function App() {
  const [flowNodes, setFlowNodes] = useState([]);
  const [wires, setWires] = useState([]);




  const [view, setView] = useState({ x: 0, y: 0, zoom: 1 });


  const [isSelecting, setIsSelecting] = useState(false);
  const selectionSnapshotRef = useRef(null);
  const additiveSelectionRef = useRef(false);
  const shiftKeyRef = useRef(false);
  const forceSingleSelectRef = useRef(null);
  const [forcedSelectedIds, setForcedSelectedIds] = useState(new Set());



  const nodes = useMemo(() => {
    return flowNodes.map(node => {
      const dataNode = node.data?.node || {};
      return {
        ...dataNode,
        id: node.id,
        x: node.position?.x ?? dataNode.x ?? 0,
        y: node.position?.y ?? dataNode.y ?? 0
      };
    });
  }, [flowNodes]);

  const selectedNodeIds = useMemo(() => {
    const ids = new Set();
    flowNodes.forEach(node => {
      if (node.selected) ids.add(node.id);
    });
    forcedSelectedIds.forEach(id => ids.add(id));
    return ids;
  }, [flowNodes, forcedSelectedIds]);


  const [activeChainIds, setActiveChainIds] = useState(new Set());




  const activeNodes = nodes.filter(n => activeChainIds.has(n.id));
  useBridge(activeNodes);

  const getViewportCenterWorld = useCallback(() => {
    const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 800;
    const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 600;
    const centerX = viewportWidth / 2;
    const centerY = viewportHeight / 2;

    return {
      worldCenterX: (centerX - view.x) / view.zoom,
      worldCenterY: (centerY - view.y) / view.zoom
    };
  }, [view.x, view.y, view.zoom]);


  const findConnectedComponent = (startNodeId) => {
    const visited = new Set();
    const queue = [startNodeId];
    visited.add(startNodeId);

    while (queue.length > 0) {
      const current = queue.shift();


      const connectedWires = wires.filter(w => w.fromNode === current || w.toNode === current);

      connectedWires.forEach(wire => {
        const neighbor = wire.fromNode === current ? wire.toNode : wire.fromNode;
        if (!visited.has(neighbor)) {
          visited.add(neighbor);
          queue.push(neighbor);
        }
      });
    }
    return visited;
  };





  const getSelectionState = () => {
    if (selectedNodeIds.size === 0) return null;


    const firstId = [...selectedNodeIds][0];
    const sequenceSet = findConnectedComponent(firstId);


    const isActive = sequenceSet.size === activeChainIds.size &&
      [...sequenceSet].every(id => activeChainIds.has(id));

    return {
      state: isActive ? 'stop' : 'start',
      nodeIds: sequenceSet
    };
  };

  const selectionContext = getSelectionState();

  const handleActivateChain = () => {
    if (!selectionContext) return;

    if (selectionContext.state === 'start') {


      setActiveChainIds(selectionContext.nodeIds);

















    } else {


      setActiveChainIds(new Set());
    }
  };



  const handleLoadChain = (chainData) => {
    if (!chainData || chainData.length === 0) return;




    const getBodyWidth = (type) => {
      const effectMeta = EFFECTS_METADATA[type];
      const paramCount = effectMeta ? Object.keys(effectMeta.params).length : 0;
      return getCardMetrics(paramCount).bodyWidth;
    };

    const GAP_BETWEEN_BODY_AND_NEXT_BODY = 120;


    let totalChainWidth = 0;
    chainData.forEach((item, index) => {
      const bodyW = getBodyWidth(item.type);
      totalChainWidth += bodyW;
      if (index < chainData.length - 1) totalChainWidth += GAP_BETWEEN_BODY_AND_NEXT_BODY;
    });


    const { worldCenterX, worldCenterY } = getViewportCenterWorld();





    let currentX = (worldCenterX - (totalChainWidth / 2)) - 50;

    const startY = worldCenterY - 100;

    const newNodes = [];
    const newWires = [];


    chainData.forEach((item) => {
      const id = Math.random().toString(36).substring(2, 15);
      const bodyWidth = getBodyWidth(item.type);


      const effectDef = EFFECTS_METADATA[item.type];
      const defaults = {};
      if (effectDef) {
        Object.keys(effectDef.params).forEach(key => {
          defaults[key] = effectDef.params[key].default;
        });
      }

      const finalParams = { ...defaults, ...item.params };

      newNodes.push({
        id,
        type: item.type,
        x: currentX,
        y: startY,
        params: finalParams
      });



      currentX += bodyWidth + GAP_BETWEEN_BODY_AND_NEXT_BODY;
    });


    for (let i = 0; i < newNodes.length - 1; i++) {
      newWires.push({
        id: Math.random().toString(36).substring(2, 15),
        fromNode: newNodes[i].id,
        toNode: newNodes[i + 1].id
      });
    }

    const newFlowNodes = newNodes.map(node => ({
      id: node.id,
      type: 'card',
      position: { x: node.x, y: node.y },
      data: { node }
    }));


    setFlowNodes(prev => [...prev, ...newFlowNodes]);
    setWires(prev => [...prev, ...newWires]);
  };

  const handleSpawnNode = (type) => {

    const effectDef = EFFECTS_METADATA[type];
    if (!effectDef) {
      return;
    }


    const initialParams = {};
    Object.keys(effectDef.params).forEach(key => {
      initialParams[key] = effectDef.params[key].default;
    });



    const id = Math.random().toString(36).substring(2, 15);





    const { worldCenterX: worldX, worldCenterY: worldY } = getViewportCenterWorld();



    const newNode = {
      id,
      type,
      x: Math.round(worldX - 60),
      y: Math.round(worldY - 90),
      params: initialParams
    };

    setFlowNodes(prev => {
      const newFlowNode = {
        id,
        type: 'card',
        position: { x: newNode.x, y: newNode.y },
        data: { node: newNode }
      };

      return [...prev, newFlowNode];
    });
  };

  const handleParamChange = (id, paramKey, newVal) => {

    setFlowNodes(prev => prev.map(node => {
      if (node.id !== id || !node.data?.node) return node;
      return {
        ...node,
        data: {
          ...node.data,
          node: {
            ...node.data.node,
            params: {
              ...node.data.node.params,
              [paramKey]: newVal
            }
          }
        }
      };
    }));



    if (activeChainIds.has(id)) {


      const activeNodesList = nodes.filter(n => activeChainIds.has(n.id));
      const sortedActive = [...activeNodesList].sort((a, b) => a.x - b.x);

      const index = sortedActive.findIndex(n => n.id === id);

      if (index !== -1 && window.ipc) {
        window.ipc.postMessage(JSON.stringify({
          type: "param_change",
          index: index,
          param_key: paramKey,
          value: newVal
        }));
      }
    }
  };

  const getNodeMetrics = useCallback((node) => {
    const effectMeta = EFFECTS_METADATA[node.type];
    const paramCount = effectMeta ? Object.keys(effectMeta.params).length : 0;
    return getCardMetrics(paramCount);
  }, []);

  const handleViewportChange = useCallback((viewport) => {
    setView({ x: viewport.x, y: viewport.y, zoom: viewport.zoom });
  }, []);

  React.useEffect(() => {
    const onKeyDown = (e) => {
      if (e.key === 'Shift') shiftKeyRef.current = true;
    };
    const onKeyUp = (e) => {
      if (e.key === 'Shift') shiftKeyRef.current = false;
    };
    const onBlur = () => {
      shiftKeyRef.current = false;
      // Embedded webviews on Windows can miss keyup after focus changes.
      // Dispatching a synthetic keyup helps ReactFlow clear stale modifier state.
      const keyup = new KeyboardEvent('keyup', { key: 'Shift' });
      window.dispatchEvent(keyup);
      document.dispatchEvent(keyup);
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
      window.removeEventListener('blur', onBlur);
    };
  }, []);

  const handleSelectionStart = useCallback((event) => {
    setIsSelecting(true);
    const additive = !!event?.shiftKey || shiftKeyRef.current;
    additiveSelectionRef.current = additive;
    if (additive) {
      const snapshot = new Set(selectedNodeIds);
      selectionSnapshotRef.current = snapshot;
      setForcedSelectedIds(snapshot);
    } else {
      selectionSnapshotRef.current = null;
      setForcedSelectedIds(new Set());
    }
  }, [selectedNodeIds]);

  const handleSelectionEnd = useCallback(() => {
    setIsSelecting(false);
    const snapshot = selectionSnapshotRef.current;
    if (additiveSelectionRef.current && snapshot && snapshot.size > 0) {
      setFlowNodes(prev => prev.map(node => ({
        ...node,
        selected: node.selected || snapshot.has(node.id)
      })));
    }
    additiveSelectionRef.current = false;
    selectionSnapshotRef.current = null;
    setForcedSelectedIds(new Set());
  }, []);

  React.useEffect(() => {
    const recoverPointerState = () => {
      if (isSelecting) {
        handleSelectionEnd();
      }
    };

    const recoverVisibilityState = () => {
      if (document.visibilityState !== 'visible') {
        shiftKeyRef.current = false;
        const keyup = new KeyboardEvent('keyup', { key: 'Shift' });
        window.dispatchEvent(keyup);
        document.dispatchEvent(keyup);
        recoverPointerState();
      }
    };

    // Some hosts/webviews occasionally drop mouseup/pointerup events.
    // These fallbacks prevent ReactFlow from getting stuck in a selection-drag state.
    window.addEventListener('mouseup', recoverPointerState, true);
    window.addEventListener('pointerup', recoverPointerState, true);
    window.addEventListener('pointercancel', recoverPointerState, true);
    window.addEventListener('dragend', recoverPointerState, true);
    window.addEventListener('blur', recoverPointerState, true);
    document.addEventListener('visibilitychange', recoverVisibilityState);

    return () => {
      window.removeEventListener('mouseup', recoverPointerState, true);
      window.removeEventListener('pointerup', recoverPointerState, true);
      window.removeEventListener('pointercancel', recoverPointerState, true);
      window.removeEventListener('dragend', recoverPointerState, true);
      window.removeEventListener('blur', recoverPointerState, true);
      document.removeEventListener('visibilitychange', recoverVisibilityState);
    };
  }, [handleSelectionEnd, isSelecting]);

  const handleNodeContextMenu = useCallback((event, node) => {
    event.preventDefault();
    setFlowNodes(prev => prev.map(n => ({
      ...n,
      selected: n.id === node.id
    })));
  }, []);

  const handleNodeClick = useCallback((event, node) => {
    if (event?.shiftKey) return;
    forceSingleSelectRef.current = node.id;
    setFlowNodes(prev => prev.map(n => ({
      ...n,
      selected: n.id === node.id
    })));
  }, []);

  const handleNodesChange = useCallback((changes) => {
    const removedIds = changes.filter(change => change.type === 'remove').map(change => change.id);
    if (removedIds.length > 0) {
      setWires(prev => prev.filter(wire => !removedIds.includes(wire.fromNode) && !removedIds.includes(wire.toNode)));
    }

    setFlowNodes(prev => {
      const UI_EXCLUSION_BOTTOM = window.innerHeight - 100;

      const adjustedChanges = changes.map(change => {
        const selectionSnapshot = selectionSnapshotRef.current;
        if (isSelecting && additiveSelectionRef.current && selectionSnapshot && change.type === 'select' && change.selected === false) {
          if (selectionSnapshot.has(change.id)) {
            return { ...change, selected: true };
          }
        }
        if (isSelecting && selectionSnapshot && change.type === 'select' && change.selected === true) {
          const existing = prev.find(node => node.id === change.id);
          const dataNode = existing?.data?.node;
          if (dataNode) {
            const { svgHeight } = getNodeMetrics(dataNode);
            const y = existing.position?.y ?? dataNode.y ?? 0;
            const nodeScreenBottom = ((y + svgHeight) * view.zoom) + view.y;

            if (nodeScreenBottom >= UI_EXCLUSION_BOTTOM) {
              return { ...change, selected: false };
            }
          }
        }
        if (isSelecting && additiveSelectionRef.current && selectionSnapshot && change.type === 'select' && change.selected === true) {
          return { ...change, selected: true };
        }
        return change;
      });

      const updated = applyNodeChanges(adjustedChanges, prev);
      let next = updated;
      const selectionSnapshot = selectionSnapshotRef.current;
      if (isSelecting && additiveSelectionRef.current && selectionSnapshot && selectionSnapshot.size > 0) {
        next = updated.map(node => ({
          ...node,
          selected: node.selected || selectionSnapshot.has(node.id)
        }));
      }
      const forcedId = forceSingleSelectRef.current;
      if (!isSelecting && forcedId) {
        next = next.map(node => ({
          ...node,
          selected: node.id === forcedId
        }));
        forceSingleSelectRef.current = null;
      }
      return next.map(node => {
        if (!node.data?.node || !node.position) return node;
        return {
          ...node,
          data: {
            ...node.data,
            node: {
              ...node.data.node,
              x: node.position.x,
              y: node.position.y
            }
          }
        };
      });
    });
  }, [getNodeMetrics, isSelecting, view.x, view.y, view.zoom]);

  const handleEdgesChange = useCallback((changes) => {
    setWires(prev => {
      let next = prev;
      changes.forEach(change => {
        if (change.type === 'remove') {
          next = next.filter(wire => wire.id !== change.id);
        }
      });
      return next;
    });
  }, []);

  const isValidConnection = useCallback((connection) => {
    if (!connection.source || !connection.target) return false;
    if (connection.source === connection.target) return false;

    const filteredWires = wires.filter(wire =>
      wire.fromNode !== connection.source && wire.toNode !== connection.target
    );

    const hasPath = (start, end, visited = new Set()) => {
      if (start === end) return true;
      if (visited.has(start)) return false;
      visited.add(start);

      const outgoing = filteredWires.filter(w => w.fromNode === start);
      for (const wire of outgoing) {
        if (hasPath(wire.toNode, end, visited)) return true;
      }
      return false;
    };

    return !hasPath(connection.target, connection.source);
  }, [wires]);

  const handleConnect = useCallback((connection) => {
    if (!connection.source || !connection.target) return;
    if (connection.source === connection.target) return;
    if (!isValidConnection(connection)) return;

    setWires(prev => {
      const cleanTarget = prev.filter(w => w.toNode !== connection.target);
      const cleanSourceAndTarget = cleanTarget.filter(w => w.fromNode !== connection.source);

      return [...cleanSourceAndTarget, {
        id: Math.random().toString(36).substring(2, 15),
        fromNode: connection.source,
        toNode: connection.target
      }];
    });
  }, [isValidConnection]);

  const renderNodes = useMemo(() => {
    return flowNodes.map(node => {
      const dataNode = node.data?.node || {};
      const mergedNode = {
        ...dataNode,
        id: node.id,
        x: node.position?.x ?? dataNode.x ?? 0,
        y: node.position?.y ?? dataNode.y ?? 0
      };

      const { svgWidth, svgHeight } = getNodeMetrics(mergedNode);

      const forcedSelected = forcedSelectedIds.has(node.id);
      return {
        ...node,
        selected: node.selected || forcedSelected,
        data: {
          ...node.data,
          node: mergedNode,
          onParamChange: handleParamChange,
          forceSelected: forcedSelected
        },
        style: {
          width: svgWidth,
          height: svgHeight
        }
      };
    });
  }, [flowNodes, forcedSelectedIds, getNodeMetrics, handleParamChange]);

  const flowEdges = useMemo(() => {
    return wires.map(wire => ({
      id: wire.id,
      source: wire.fromNode,
      target: wire.toNode,
      type: 'wire',
      selectable: false,
      data: {
        strokeWidth: 2,
        sourceRadius: 12.5,
        targetRadius: 12.5
      }
    }));
  }, [wires]);


  React.useEffect(() => {
    const handleKeyDown = (e) => {

      if (e.target.matches('input, textarea')) return;


      if (e.key === '1') {

        handleSpawnNode('Overdrive');
      }
      if (e.key === '2') {

        handleSpawnNode('Delay');
      }

      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedNodeIds.size > 0) {

          setFlowNodes(prev => prev.filter(n => !selectedNodeIds.has(n.id)));

          setWires(prev => prev.filter(w => !selectedNodeIds.has(w.fromNode) && !selectedNodeIds.has(w.toNode)));



          requestAnimationFrame(() => {
            const root = document.body;
            if (root) {

              root.style.transform = 'translateZ(0.1px)';
              requestAnimationFrame(() => {
                root.style.transform = '';
              });
            }
          });
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectedNodeIds]);


  const [isInterfaceHovered, setIsInterfaceHovered] = useState(false);
  const [showUpdateNotice, setShowUpdateNotice] = useState(false);
  const [updateNoticeText, setUpdateNoticeText] = useState('Update available. Click to install.');
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const latestUpdateInfoRef = useRef(null);

  React.useEffect(() => {
    let mounted = true;
    const controller = new AbortController();

    const checkForUpdate = async () => {
      try {
        const updateInfo = await fetchPluginUpdateInfo({ signal: controller.signal });
        if (!mounted) return;

        latestUpdateInfoRef.current = updateInfo;

        if (!updateInfo.updateAvailable) {
          setShowUpdateNotice(false);
          return;
        }

        const criticalPrefix = updateInfo.isCritical ? 'Critical update' : 'Update';
        const versionPart = updateInfo.latestVersion ? ` ${updateInfo.latestVersion}` : '';
        setUpdateNoticeText(`${criticalPrefix}${versionPart} available. Click to install.`);
        setShowUpdateNotice(true);
      } catch (error) {
        if (error?.name === 'AbortError') return;
        console.warn('[update-check] Failed to fetch latest plugin version', error);
        if (mounted) {
          setShowUpdateNotice(false);
        }
      }
    };

    checkForUpdate();

    return () => {
      mounted = false;
      controller.abort();
    };
  }, []);

  const handleUserUpdate = useCallback(async () => {
    if (isInstallingUpdate) return;

    setIsInstallingUpdate(true);
    setUpdateNoticeText('Opening installer...');

    try {
      const result = await installPluginUpdate(latestUpdateInfoRef.current || {});
      setUpdateNoticeText(result.message || 'Installer opened. Complete update and restart Tonelab.');
    } catch (error) {
      setUpdateNoticeText(error?.message || 'Failed to open installer. Click to retry.');
    } finally {
      setIsInstallingUpdate(false);
    }
  }, [isInstallingUpdate]);

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        backgroundColor: '#0a0a0a',
        overflow: 'hidden',
        position: 'relative'
      }}
    >
      <div style={{ position: 'absolute', top: 5, left: 5, color: 'lime', zIndex: 9999, pointerEvents: 'none' }}></div>

      {showUpdateNotice && (
        <UpdateNotice
          onClick={handleUserUpdate}
          text={updateNoticeText}
          disabled={isInstallingUpdate}
        />
      )}

      <Dashboard
        flowNodes={renderNodes}
        flowEdges={flowEdges}
        view={view}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={handleConnect}
        isValidConnection={isValidConnection}
        onSelectionStart={handleSelectionStart}
        onSelectionEnd={handleSelectionEnd}
        onNodeClick={handleNodeClick}
        onNodeContextMenu={handleNodeContextMenu}
        onViewportChange={handleViewportChange}
      />

      <div
        style={{
          position: 'fixed',
          bottom: '30px',
          left: '50%',
          transform: 'translateX(-50%)',
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          zIndex: 100,
          pointerEvents: isSelecting ? 'none' : 'none',
          isolation: 'isolate',
          willChange: 'contents'
        }}
        onMouseEnter={() => !isSelecting && setIsInterfaceHovered(true)}
        onMouseLeave={() => !isSelecting && setIsInterfaceHovered(false)}
      >
        <div style={{

          pointerEvents: isSelecting ? 'none' : 'auto',

          transform: 'translateZ(0)',
          isolation: 'isolate'
        }}>
          <Toolbar
            onSpawn={handleSpawnNode}
            onLoadChain={handleLoadChain}
            isHovered={isInterfaceHovered}
          />
        </div>

        {selectionContext && (
          <div style={{

            pointerEvents: isSelecting ? 'none' : 'auto',

            transform: 'translateZ(0)',
            isolation: 'isolate'
          }}>
            <ActivationButton
              state={selectionContext.state}
              onClick={handleActivateChain}
              isHovered={isInterfaceHovered}
            />
          </div>
        )}
      </div>


    </div>
  );
}


export default App;
