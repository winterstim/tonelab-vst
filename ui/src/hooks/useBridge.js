import { useEffect, useRef } from 'react';
import { postIpcMessage } from '../utils/ipcBridge';

export function useBridge(nodes) {
    const lastSentRef = useRef("");

    useEffect(() => {

        const sortedNodes = [...nodes].sort((a, b) => a.x - b.x);



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

        const payload = {
            type: 'sync_chain',
            data: chain
        };
        const json = JSON.stringify(payload);

        if (json === lastSentRef.current) return;
        lastSentRef.current = json;


        postIpcMessage(payload);

    }, [nodes]);
}
