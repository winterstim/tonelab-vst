import React from 'react';

export const Tooltip = ({ text }) => (
    <div style={{
        position: 'absolute',
        bottom: '140%',
        left: '50%',
        transform: 'translateX(-50%)',
        padding: '6px 10px',
        background: 'var(--toolbar-popup-bg)',
        border: '1px solid var(--dashboard-border)',
        borderRadius: '6px',
        color: 'white',
        fontSize: '12px',
        fontWeight: '500',
        whiteSpace: 'nowrap',
        pointerEvents: 'none',
        backdropFilter: 'blur(var(--liquid-glass-blur)) saturate(135%)',
        WebkitBackdropFilter: 'blur(var(--liquid-glass-blur)) saturate(135%)',
        opacity: 1,
        animation: 'fadeIn 0.2s ease-out',
        boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
        zIndex: 1000
    }}>
        {text}
        <style>{`
            @keyframes fadeIn {
                from { opacity: 0; transform: translateX(-50%) translateY(4px); }
                to { opacity: 1; transform: translateX(-50%) translateY(0); }
            }
        `}</style>
    </div>
);
