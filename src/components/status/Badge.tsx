import classNames from "classnames";
import { ReactNode } from "react";

interface BadgeProps {
  label: string;
  tone?: "neutral" | "success" | "warning" | "danger";
  icon?: ReactNode;
}

function Badge({ label, tone = "neutral", icon }: BadgeProps) {
  return (
    <span className={classNames("badge", tone)}>
      {icon && <span className="badge-icon">{icon}</span>}
      {label}
    </span>
  );
}

export default Badge;
