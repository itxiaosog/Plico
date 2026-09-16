import type { ReactNode } from "react";

import type { ItemType } from "../types";

const PIXEL_ICONS: Record<ItemType, ReactNode> = {
  text: (
    <>
      <rect x="2" y="3" width="8" height="1.5" />
      <rect x="2" y="6.25" width="8" height="1.5" />
      <rect x="2" y="9.5" width="5" height="1.5" />
    </>
  ),
  rich_text: (
    <>
      <rect x="2" y="3" width="8" height="1.5" />
      <rect x="2" y="6.25" width="5" height="1.5" />
      <rect x="8" y="6.25" width="2" height="1.5" />
      <rect x="2" y="9.5" width="6" height="1.5" />
    </>
  ),
  image: (
    <>
      <rect x="2" y="3" width="10" height="8" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <rect x="4" y="5" width="2" height="2" />
      <path d="M3 10 L5.5 7 L7 8.5 L9 6 L11 10 Z" />
    </>
  ),
  files: (
    <>
      <path d="M2 4 L2 11 L12 11 L12 5 L7 5 L6 4 Z" />
    </>
  ),
  link: (
    <>
      <rect x="2" y="2" width="4" height="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <rect x="8" y="8" width="4" height="4" fill="none" stroke="currentColor" strokeWidth="1.5" />
      <path d="M6 4 L8 4 L8 10 L6 10" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </>
  ),
  color: (
    <>
      <path d="M7 2 C7 2 3 6 3 9 C3 11 5 12 7 12 C9 12 11 11 11 9 C11 6 7 2 7 2 Z" />
    </>
  ),
};

interface Props {
  type: ItemType;
  size?: number;
}

export function TypeIcon({ type, size = 14 }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      fill="currentColor"
      aria-hidden="true"
    >
      {PIXEL_ICONS[type]}
    </svg>
  );
}

export function PinIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="4" y="1" width="4" height="1.5" />
      <rect x="5.25" y="2.5" width="1.5" height="3.5" />
      <rect x="3.5" y="6" width="5" height="2" />
      <rect x="5.25" y="8" width="1.5" height="3" />
    </svg>
  );
}

export function SearchIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="square"
      aria-hidden="true"
    >
      <circle cx="6" cy="6" r="3.5" />
      <path d="M9 9 L12 12" />
    </svg>
  );
}

export function TrashIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="1.5" y="2.5" width="9" height="1.5" />
      <rect x="4" y="1" width="4" height="1.5" />
      <rect x="3" y="4" width="1.5" height="7" />
      <rect x="5.25" y="4" width="1.5" height="7" />
      <rect x="7.5" y="4" width="1.5" height="7" />
    </svg>
  );
}

export function PencilIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="8.5" y="1" width="2.5" height="2.5" />
      <rect x="6.5" y="3" width="2.5" height="2.5" />
      <rect x="4.5" y="5" width="2.5" height="2.5" />
      <rect x="2.5" y="7" width="2.5" height="2.5" />
      <rect x="1" y="9" width="1.5" height="1.5" />
    </svg>
  );
}

export function TuneIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="currentColor"
      aria-hidden="true"
    >
      <rect x="1" y="2.5" width="7" height="1.5" />
      <rect x="1" y="8" width="7" height="1.5" />
      <rect x="3.5" y="1.5" width="2" height="3.5" />
      <rect x="6.5" y="7" width="2" height="3.5" />
    </svg>
  );
}

export function CheckIcon({ size = 12 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden="true"
    >
      <path d="M2 6 L5 9 L10 3" />
    </svg>
  );
}

export function ChevronIcon({ dir, size = 12 }: { dir: "left" | "right"; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden="true"
    >
      <path d={dir === "right" ? "M4 2 L8 6 L4 10" : "M8 2 L4 6 L8 10"} />
    </svg>
  );
}
