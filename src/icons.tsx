// One stroke family, drawn for this app: 24-unit grid, 1.5 stroke, square caps.

const PATHS = {
  play: "M8 5.5v13l10.5-6.5z",
  pause: "M8 5v14M16 5v14",
  upload: "M12 16V4m0 0L7 9m5-5 5 5M4 15v4.5h16V15",
  download: "M12 4v12m0 0-5-5m5 5 5-5M4 15v4.5h16V15",
  cut: "M6 7a2.5 2.5 0 1 0 0-.01M6 17a2.5 2.5 0 1 0 0 .01M8.2 8.3 20 18M8.2 15.7 20 6",
  trash: "M4 7h16M9 7V4.5h6V7m-8.5 0 1 12.5h9l1-12.5M10 11v5m4-5v5",
  close: "M6 6l12 12M18 6 6 18",
  copy: "M8.5 8.5h11v11h-11zM15.5 8.5v-4h-11v11h4",
  check: "M5 12.5l4.5 4.5L19 7.5",
  sun: "M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zM12 2.5v2.5m0 14v2.5M2.5 12H5m14 0h2.5M5.3 5.3l1.8 1.8m9.8 9.8 1.8 1.8m0-13.4-1.8 1.8m-9.8 9.8-1.8 1.8",
  moon: "M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z",
  search: "M10.5 4.5a6 6 0 1 0 0 12 6 6 0 0 0 0-12zM15 15l5 5",
  undo: "M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11",
  edit: "M4.5 19.5h4l10-10-4-4-10 10zM13 7l4 4",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 18, label }: { name: IconName; size?: number; label?: string }) {
  return (
    <svg
      className="icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={name === "play" ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden={label ? undefined : true}
      role={label ? "img" : undefined}
      aria-label={label}
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
