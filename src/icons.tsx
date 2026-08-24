// Minimal inline SVG icon set (stroke-based, 24x24 viewBox).
type IconProps = { size?: number; className?: string };

const base = (size = 18) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
});

export function IconChip({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <rect x="7" y="7" width="10" height="10" rx="2" />
      <path d="M9 1v2M15 1v2M9 21v2M15 21v2M1 9h2M1 15h2M21 9h2M21 15h2" />
      <rect x="9.5" y="9.5" width="5" height="5" rx="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function IconDrive({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <rect x="2" y="7" width="20" height="10" rx="2" />
      <circle cx="17.5" cy="12" r="1.4" fill="currentColor" stroke="none" />
      <path d="M6 12h7" />
    </svg>
  );
}

export function IconCache({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <circle cx="12" cy="12" r="9" strokeDasharray="4 3" />
      <path d="M12 7v5l3.5 2" />
    </svg>
  );
}

export function IconBolt({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M13 2L4 14h6l-1 8 9-12h-6l1-8z" />
    </svg>
  );
}

export function IconSettings({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M19 12a7 7 0 00-.15-1.4l2-1.55-2-3.46-2.35.95A7 7 0 0015 5.15L14.5 2.7h-4l-.5 2.45a7 7 0-1.65 1.39l-2.35-.95-2 3.46 2 1.55a7 7 0 000 2.8l-2 1.55 2 3.46 2.35-.95a7 7 0 001.65 1.39l.5 2.45h4l.5-2.45a7 7 0 001.65-1.39l2.35.95 2-3.46-2-1.55c.1-.46.15-.93.15-1.4z" />
    </svg>
  );
}

export function IconGauge({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M4 18a8 8 0 1116 0" />
      <path d="M12 18l4-6" />
      <circle cx="12" cy="18" r="1.2" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function IconShield({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M12 2l8 3.5v5.5c0 5-3.5 8.5-8 11-4.5-2.5-8-6-8-11V5.5L12 2z" />
      <path d="M9 12l2 2 4-4" />
    </svg>
  );
}

export function IconPalette({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M12 2a10 10 0 100 20c1.7 0 2.4-1.3 1.7-2.6-.6-1.1.2-2.4 1.4-2.4H17a5 5 0 005-5c0-5.5-4.5-10-10-10z" />
      <circle cx="7.5" cy="11.5" r="1.2" fill="currentColor" stroke="none" />
      <circle cx="10.5" cy="7.5" r="1.2" fill="currentColor" stroke="none" />
      <circle cx="15" cy="7.5" r="1.2" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function IconBell({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M6 9a6 6 0 1112 0c0 5 2 6 2 6H4s2-1 2-6" />
      <path d="M10 20a2 2 0 004 0" />
    </svg>
  );
}

export function IconTray({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <rect x="3" y="3" width="18" height="18" rx="4" />
      <circle cx="12" cy="12" r="4" />
    </svg>
  );
}

export function IconKeyboard({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <rect x="2" y="7" width="20" height="11" rx="2" />
      <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M18 14h.01M9 14h6" />
    </svg>
  );
}

export function IconSparkles({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <path d="M12 3l1.8 4.8L19 9.5l-5.2 1.7L12 16l-1.8-4.8L5 9.5l5.2-1.7L12 3z" />
      <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8L19 15z" />
    </svg>
  );
}

export function IconInfo({ size, className }: IconProps) {
  return (
    <svg {...base(size)} className={className}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5" />
      <path d="M12 8h.01" />
    </svg>
  );
}
