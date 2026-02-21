import { JSX } from "solid-js";
import { twMerge } from "tailwind-merge";

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {}

function Button({ type = "button", class: className, disabled, onClick, children }: ButtonProps) {
  const classes = twMerge(
    "rounded bg-zinc-900 px-3 py-1.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60",
    className,
  );
  return (
    <button type={type} class={classes} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  );
}

export default Button;
