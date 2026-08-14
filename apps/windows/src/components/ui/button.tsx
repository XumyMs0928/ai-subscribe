import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";

import { cn } from "../../lib/utils";

const buttonVariants = cva(
    "inline-flex min-h-10 items-center justify-center rounded-control border border-transparent px-4 py-2 text-sm font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:pointer-events-none disabled:opacity-50 motion-reduce:transition-none",
    {
        variants: {
            variant: {
                primary:
                    "bg-accent text-accent-foreground hover:bg-accent-strong",
                secondary:
                    "border-border bg-surface text-foreground hover:bg-surface-muted",
            },
        },
        defaultVariants: { variant: "primary" },
    },
);

export interface ButtonProps
    extends
        React.ButtonHTMLAttributes<HTMLButtonElement>,
        VariantProps<typeof buttonVariants> {
    readonly asChild?: boolean;
}

export function Button({
    asChild = false,
    className,
    variant,
    ...props
}: ButtonProps) {
    const Component = asChild ? Slot : "button";
    return (
        <Component
            className={cn(buttonVariants({ variant }), className)}
            {...props}
        />
    );
}
