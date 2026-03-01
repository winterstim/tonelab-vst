import './UpdateNotice.css';

export default function UpdateNotice({
    onClick,
    text = 'Update available. Click to install.',
    disabled = false
}) {
    return (
        <div className="update-notice-wrapper">
            <button
                type="button"
                className="update-notice liquid-glass-surface liquid-glass-toolbar-surface"
                disabled={disabled}
                onMouseDown={(event) => event.stopPropagation()}
                onClick={(event) => {
                    event.stopPropagation();
                    if (disabled) return;
                    if (onClick) onClick();
                }}
                aria-label={text}
            >
                {text}
            </button>
        </div>
    );
}
