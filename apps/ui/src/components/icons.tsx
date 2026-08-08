import type { SVGProps } from 'react';

type IconProps = SVGProps<SVGSVGElement>;

const base = {
  width: 16,
  height: 16,
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.7,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

export const OverviewIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <rect x="3" y="3" width="7" height="9" rx="1" />
    <rect x="14" y="3" width="7" height="5" rx="1" />
    <rect x="14" y="12" width="7" height="9" rx="1" />
    <rect x="3" y="16" width="7" height="5" rx="1" />
  </svg>
);

export const TrafficIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M4 6c2-2 6-2 8 0s6 2 8 0" />
    <path d="M4 12c2-2 6-2 8 0s6 2 8 0" />
    <path d="M4 18c2-2 6-2 8 0s6 2 8 0" />
  </svg>
);

export const ModelsIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <rect x="3" y="3" width="7" height="7" rx="1" />
    <rect x="14" y="3" width="7" height="7" rx="1" />
    <rect x="14" y="14" width="7" height="7" rx="1" />
    <rect x="3" y="14" width="7" height="7" rx="1" />
  </svg>
);

export const KeysIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <circle cx="8" cy="15" r="4" />
    <path d="m10.8 12.2 8-8" />
    <path d="M15 9l3 3" />
    <path d="M16.5 7.5 19 10" />
  </svg>
);

export const SettingsIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M10 3 9 6H5l-1 4h4l-1 3-3-1-2 4 5 2 .7 3h4l-.6-3 3-1 .6 3h3l.6-3 4-2-2-4-3 1 1-3h4l-1-4h-4L13 3h-3z" />
    <circle cx="13" cy="12" r="2" />
  </svg>
);

export const ProvidersIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <circle cx="12" cy="5" r="2.5" />
    <circle cx="5" cy="19" r="2.5" />
    <circle cx="19" cy="19" r="2.5" />
    <path d="M12 7.5 5.5 16.5" />
    <path d="m12 7.5 6.5 9" />
  </svg>
);

export const SearchIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <circle cx="11" cy="11" r="7" />
    <path d="m21 21-4.3-4.3" />
  </svg>
);

export const BellIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={1.8} {...p}>
    <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
    <path d="M13.7 21a2 2 0 0 1-3.4 0" />
  </svg>
);

export const KeyboardIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={1.8} {...p}>
    <rect x="2" y="6" width="20" height="12" rx="2" />
    <path d="M6 10h0M10 10h0M14 10h0M18 10h0M6 14h0M10 14h0M14 14h4" />
  </svg>
);

export const PlusIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2.2} {...p}>
    <path d="M12 5v14M5 12h14" />
  </svg>
);

export const CalendarIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <rect x="3" y="4" width="18" height="18" rx="2" />
    <path d="M16 2v4M8 2v4M3 10h18" />
  </svg>
);

export const DownloadIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <path d="M12 3v12" />
    <path d="m7 10 5 5 5-5" />
    <path d="M4 21h16" />
  </svg>
);

export const CloseIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <path d="M18 6 6 18M6 6l12 12" />
  </svg>
);

export const ArrowUpIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <path d="m6 9 6-6 6 6" />
    <path d="M12 3v18" />
  </svg>
);

export const ArrowDownIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <path d="m6 15 6 6 6-6" />
    <path d="M12 21V3" />
  </svg>
);

export const ExportIcon = (p: IconProps) => (
  <svg {...base} strokeWidth={2} {...p}>
    <path d="M12 3v18" />
    <path d="M5 10h14M5 14h10" />
  </svg>
);

export const BoxIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M21 8 12 3 3 8v8l9 5 9-5V8z" />
    <path d="M3 8l9 5 9-5" />
    <path d="M12 13v8" />
  </svg>
);

export const BoltIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M13 2 4 14h6l-1 8 9-12h-6l1-8z" />
  </svg>
);

export const LogoMark = (p: IconProps) => (
  <svg {...base} strokeWidth={2.2} {...p}>
    <path d="M13 2 4 14h6l-1 8 9-12h-6l1-8z" />
  </svg>
);
