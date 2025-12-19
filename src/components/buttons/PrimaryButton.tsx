import classNames from "classnames";
import { ButtonHTMLAttributes, ReactNode } from "react";

interface PrimaryButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  loading?: boolean;
  iconLeft?: ReactNode;
}

function PrimaryButton({
  variant = "primary",
  loading,
  disabled,
  children,
  iconLeft,
  ...rest
}: PrimaryButtonProps) {
  return (
    <button
      className={classNames("btn", variant, { loading })}
      disabled={disabled || loading}
      {...rest}
    >
      {iconLeft && <span className="btn-icon">{iconLeft}</span>}
      <span>{loading ? "Loading..." : children}</span>
    </button>
  );
}

export default PrimaryButton;
