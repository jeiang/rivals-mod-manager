import { JSX } from "solid-js";
import { splitProps } from "solid-js";
import { twMerge } from "tailwind-merge";

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {}

function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["class", "type"]);
  const classes = twMerge(
    "rounded bg-zinc-900 px-3 py-1.5 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60",
    local.class,
  );

  return <button type={local.type ?? "button"} class={classes} {...rest} />;
}

export default Button;
