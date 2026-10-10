// Canonical route metadata (§18) — THE single source of truth for the shell.
// Sidebar (NAV), command palette (buildPaletteCommands), currentRoute() and
// the topbar breadcrumb all derive from ROUTES below; App.tsx owns no route
// literals of its own. The Route union carries NO 'tools': the placeholder
// route was removed as unreachable (§16, variant B — INTENTIONALLY_REJECTED;
// an external #/tools hash falls back to 'home' like any unknown hash).
//
// Ordering invariant: ROUTES order is chosen so that
//   - the palette projection (visibleInPalette, in order) keeps the EXACT
//     composition and order pinned by palette-acceptance (WDIO): 14 nav
//     commands, labels resolved through paletteKey (byte-for-byte the old
//     palette.cmd.* labels), and
//   - the sidebar projection (visibleInSidebar, settings rendered in the
//     sidebar-bottom slot) keeps the historical NAV order.
import {
  ArrowRightLeft,
  BookOpen,
  Database,
  FolderOpen,
  GitCompareArrows,
  Globe,
  Languages,
  LifeBuoy,
  MessagesSquare,
  Package,
  PanelsTopLeft,
  Plug,
  ShieldCheck,
  Settings2,
  type LucideIcon,
} from 'lucide-react'

export type Route =
  | 'home'
  | 'projects'
  | 'workspace'
  | 'checks'
  | 'existing'
  | 'compare'
  | 'glossary'
  | 'tm'
  | 'chatbatch'
  | 'export'
  | 'providers'
  | 'selfloc'
  | 'diagnostics'
  | 'lm'
  | 'settings'

export interface RouteMeta {
  route: Route
  /** Honest label of the route (sidebar / breadcrumb). Providers, selfloc
   *  and diagnostics resolve to their OWN labels — never a Settings
   *  fallback (§18). */
  labelKey: string
  /** Palette label key, pinned byte-for-byte by palette-acceptance (WDIO);
   *  several differ from the nav wording («Импорт существующего перевода»
   *  vs «Существующий перевод»). Falls back to labelKey when absent. */
  paletteKey?: string
  icon: LucideIcon
  visibleInSidebar: boolean
  visibleInPalette: boolean
  /** Historical primary-nav position (Lovable R1 canon: «Проекты» first,
   *  «Строки перевода» second — while the palette pins «Строки перевода»
   *  FIRST). One array cannot preserve both orders via filtering, so the
   *  sidebar projection sorts by this rank; palette keeps array order.
   *  Absent for settings (bottom slot) and non-sidebar routes. */
  sidebarOrder?: number
  /** The surface is meaningful only over an open project (its screens show
   *  an honest needProject notice otherwise); informational for guards. */
  requiresProject: boolean
}

export const ROUTES: RouteMeta[] = [
  {
    route: 'home',
    labelKey: 'nav.entries',
    paletteKey: 'palette.cmd.entries',
    icon: Languages,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: false,
    sidebarOrder: 2,
  },
  {
    route: 'projects',
    labelKey: 'nav.projects',
    paletteKey: 'palette.cmd.projects',
    icon: FolderOpen,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: false,
    sidebarOrder: 1,
  },
  {
    // Reached from Home/Open; the sidebar entry for it IS 'home' (the nav
    // highlights home while in workspace — App keeps that pairing).
    route: 'workspace',
    labelKey: 'nav.entries',
    icon: PanelsTopLeft,
    visibleInSidebar: false,
    visibleInPalette: false,
    requiresProject: true,
  },
  {
    route: 'checks',
    labelKey: 'nav.checks',
    paletteKey: 'palette.cmd.checks',
    icon: ShieldCheck,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 3,
  },
  {
    route: 'existing',
    labelKey: 'nav.existing',
    paletteKey: 'palette.cmd.existing',
    // D-V2: unique icon per route — compare keeps GitCompareArrows; the
    // existing-import flow is a hand-off (arrow swap), not a diff.
    icon: ArrowRightLeft,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 4,
  },
  {
    route: 'compare',
    labelKey: 'nav.compare',
    paletteKey: 'palette.cmd.compare',
    icon: GitCompareArrows,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: false,
    sidebarOrder: 5,
  },
  {
    route: 'glossary',
    labelKey: 'nav.glossary',
    paletteKey: 'palette.cmd.glossary',
    // D-V2: the ref icon language — one distinct icon per route
    // (Глоссарий=BookOpen, Сборка=Package, TM=Database).
    icon: BookOpen,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 6,
  },
  {
    route: 'tm',
    labelKey: 'nav.tm',
    paletteKey: 'palette.cmd.tm',
    icon: Database,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 7,
  },
  {
    route: 'chatbatch',
    labelKey: 'nav.chatbatch',
    paletteKey: 'palette.cmd.chatbatch',
    icon: MessagesSquare,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 8,
  },
  {
    route: 'export',
    labelKey: 'nav.export',
    paletteKey: 'palette.cmd.export',
    // D-V2: Сборка=Package (the ref mapping; Wrench duplicated no one but
    // lost the ship/box metaphor the screen is about).
    icon: Package,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: true,
    sidebarOrder: 9,
  },
  {
    route: 'selfloc',
    labelKey: 'nav.selfloc',
    paletteKey: 'palette.cmd.selfloc',
    icon: Globe,
    visibleInSidebar: false,
    visibleInPalette: true,
    requiresProject: false,
  },
  {
    route: 'diagnostics',
    labelKey: 'nav.diagnostics',
    paletteKey: 'palette.cmd.diagnostics',
    icon: LifeBuoy,
    visibleInSidebar: false,
    visibleInPalette: true,
    requiresProject: true,
  },
  {
    route: 'providers',
    labelKey: 'nav.providers',
    paletteKey: 'palette.cmd.providers',
    icon: Plug,
    visibleInSidebar: false,
    visibleInPalette: true,
    requiresProject: false,
  },
  {
    route: 'lm',
    labelKey: 'nav.lm',
    paletteKey: 'palette.cmd.lm',
    icon: Globe,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: false,
    sidebarOrder: 10,
  },
  {
    // Sidebar placement is the bottom slot (App renders it separately from
    // primary-nav) — still "visible in the sidebar".
    route: 'settings',
    labelKey: 'nav.settings',
    paletteKey: 'palette.cmd.settings',
    icon: Settings2,
    visibleInSidebar: true,
    visibleInPalette: true,
    requiresProject: false,
  },
]

/** All known route ids — the whitelist behind currentRoute(); an unknown
 *  hash resolves to 'home'. */
export const ROUTE_IDS: Route[] = ROUTES.map((m) => m.route)

export function routeMeta(route: Route): RouteMeta | undefined {
  return ROUTES.find((m) => m.route === route)
}
