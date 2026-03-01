import { EFFECTS_METADATA, validateParam } from '../config/effects';




export const serializeChain = (nodes, wires) => {




    const allToNodes = new Set(wires.map(w => w.toNode));
    let startNode = nodes.find(n => !allToNodes.has(n.id));

    if (!startNode && nodes.length > 0) {

        startNode = nodes[0];
    }

    const chain = [];
    let currentNode = startNode;
    const visited = new Set();

    while (currentNode) {
        if (visited.has(currentNode.id)) break;
        visited.add(currentNode.id);


        const cleanParams = {};
        const meta = EFFECTS_METADATA[currentNode.type];

        if (meta) {
            Object.keys(meta.params).forEach(key => {
                const val = currentNode.params[key];


                if (!validateParam(currentNode.type, key, val)) {
                    throw new Error(`Invalid Value for ${meta.label}.${key}: ${val}`);
                }

                cleanParams[key] = val;
            });


            chain.push({
                type: currentNode.type,
                params: cleanParams
            });
        }


        const outgoingWire = wires.find(w => w.fromNode === currentNode.id);
        if (outgoingWire) {
            currentNode = nodes.find(n => n.id === outgoingWire.toNode);
        } else {
            currentNode = null;
        }
    }

    return chain;
};
