import classNames from "classnames";
import { InputHTMLAttributes } from "react";

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helperText?: string;
  fullWidth?: boolean;
}

function TextField({ label, helperText, fullWidth, className, ...rest }: TextFieldProps) {
  return (
    <label className={classNames("textfield", { fullWidth })}>
      {label && <span className="textfield-label">{label}</span>}
      <input className={classNames("textfield-input", className)} {...rest} />
      {helperText && <span className="textfield-helper">{helperText}</span>}
    </label>
  );
}

export default TextField;
