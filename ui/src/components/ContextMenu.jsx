import React, { useEffect, useRef } from 'react';

export default function ContextMenu({ x, y, items, onClose }) {
    const menuRef = useRef(null);

    useEffect(() => {
        const handleClickOutside = (e) => {
            if (menuRef.current && !menuRef.current.contains(e.target)) {
                onClose();
            }
        };

        requestAnimationFrame(() => {
            window.addEventListener('mousedown', handleClickOutside, true);
        });

        return () => window.removeEventListener('mousedown', handleClickOutside, true);
    }, [onClose]);

    return (
        <div
            ref={menuRef}
            style={{
                position: 'fixed',
                top: y,
                left: x,
                background: 'var(--toolbar-popup-bg)',
                backdropFilter: 'blur(var(--liquid-glass-blur)) saturate(135%)',
                WebkitBackdropFilter: 'blur(var(--liquid-glass-blur)) saturate(135%)',
                border: '1px solid var(--dashboard-border)',
                borderRadius: '8px',
                padding: '4px',
                boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
                zIndex: 99999,
                display: 'flex',
                flexDirection: 'column',
                minWidth: '130px'
            }}
            onContextMenu={(e) => { e.preventDefault(); e.stopPropagation(); }}
        >
            {items.map((item, i) => (
                <div
                    key={i}
                    onClick={(e) => {
                        e.stopPropagation();
                        item.onClick();
                        onClose();
                    }}
                    style={{
                        padding: '10px 14px',
                        color: item.color || '#e0e0e0',
                        fontSize: '14px',
                        fontFamily: 'Inter, sans-serif',
                        cursor: 'pointer',
                        borderRadius: '4px',
                        userSelect: 'none',
                        transition: 'background 0.1s'
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.background = 'var(--dashboard-border)'}
                    onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
                >
                    {item.label}
                </div>
            ))}
        </div>
    );
}
