interface ProgressBarProps {
  value: number; // 0-1
  status?: string;
}

function ProgressBar({ value, status }: ProgressBarProps) {
  const pct = Math.min(1, Math.max(0, value));
  return (
    <div className="progress-bar">
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${pct * 100}%` }} />
      </div>
      {status && <div className="progress-status">{status}</div>}
    </div>
  );
}

export default ProgressBar;
