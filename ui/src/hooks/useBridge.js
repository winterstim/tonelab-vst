import { useEffect, useRef } from 'react';

export function useBridge(nodes) {
    const lastSentRef = useRef("");

    useEffect(() => {

        const sortedNodes = [...nodes].sort((a, b) => a.x - b.x);



        const topology = sortedNodes.map(n => ({
            type: n.type,
            id: n.id

        }));

        const topologyJson = JSON.stringify(topology);


        if (topologyJson === lastSentRef.current) return;
        lastSentRef.current = topologyJson;


        const chain = sortedNodes.map(node => {
            const cleanParams = {};

            Object.keys(node.params).forEach(key => {
                const val = node.params[key];

                cleanParams[key] = val;
            });

            return {
                type: node.type,
                params: cleanParams
            };
        });

        const json = JSON.stringify(chain);


        if (window.ipc) {
            window.ipc.postMessage(json);
        }

    }, [nodes]);
}
