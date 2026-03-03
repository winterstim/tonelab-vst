export function openExternalUrl(url) {
    if (typeof window !== 'undefined' && window.ipc?.postMessage) {
        window.ipc.postMessage(JSON.stringify({
            type: 'open_external_url',
            url
        }));
        return;
    }

    if (typeof window !== 'undefined' && typeof window.open === 'function') {
        window.open(url, '_blank', 'noopener,noreferrer');
        return;
    }

    throw new Error('Unable to open external URL');
}
