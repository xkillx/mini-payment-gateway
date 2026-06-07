---
name: Modern Logic
colors:
  surface: '#f7f9fc'
  surface-dim: '#d8dadd'
  surface-bright: '#f7f9fc'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f2f4f7'
  surface-container: '#eceef1'
  surface-container-high: '#e6e8eb'
  surface-container-highest: '#e0e3e6'
  on-surface: '#191c1e'
  on-surface-variant: '#454652'
  inverse-surface: '#2d3133'
  inverse-on-surface: '#eff1f4'
  outline: '#767683'
  outline-variant: '#c6c5d4'
  surface-tint: '#4c56af'
  primary: '#000666'
  on-primary: '#ffffff'
  primary-container: '#1a237e'
  on-primary-container: '#8690ee'
  inverse-primary: '#bdc2ff'
  secondary: '#006b5f'
  on-secondary: '#ffffff'
  secondary-container: '#8df5e4'
  on-secondary-container: '#007165'
  tertiary: '#271800'
  on-tertiary: '#ffffff'
  tertiary-container: '#422c00'
  on-tertiary-container: '#c98c00'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#e0e0ff'
  primary-fixed-dim: '#bdc2ff'
  on-primary-fixed: '#000767'
  on-primary-fixed-variant: '#343d96'
  secondary-fixed: '#8df5e4'
  secondary-fixed-dim: '#70d8c8'
  on-secondary-fixed: '#00201c'
  on-secondary-fixed-variant: '#005048'
  tertiary-fixed: '#ffdeac'
  tertiary-fixed-dim: '#ffba38'
  on-tertiary-fixed: '#281900'
  on-tertiary-fixed-variant: '#604100'
  background: '#f7f9fc'
  on-background: '#191c1e'
  surface-variant: '#e0e3e6'
typography:
  display-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 48px
    fontWeight: '700'
    lineHeight: 56px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 32px
    fontWeight: '600'
    lineHeight: 40px
    letterSpacing: -0.01em
  headline-lg-mobile:
    fontFamily: Plus Jakarta Sans
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  headline-md:
    fontFamily: Plus Jakarta Sans
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  body-lg:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '400'
    lineHeight: 28px
  body-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  body-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  label-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '600'
    lineHeight: 16px
    letterSpacing: 0.01em
  label-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 14px
    letterSpacing: 0.02em
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  base: 8px
  container-max: 1280px
  gutter: 24px
  margin-desktop: 40px
  margin-mobile: 16px
  stack-xs: 4px
  stack-sm: 12px
  stack-md: 24px
  stack-lg: 48px
---

## Brand & Style
The design system is engineered for high-stakes financial environments, prioritizing clarity, security, and velocity. The brand personality is "The Reliable Partner"—authoritative yet accessible, removing friction from complex transactional data. 

The aesthetic follows a **Corporate / Modern** direction with a focus on precision. It utilizes a structured hierarchy, ample negative space to reduce cognitive load during payment processing, and subtle depth cues to guide user intent. The visual language conveys institutional stability through a heavy reliance on a disciplined grid and a sophisticated, cool-toned palette.

## Colors
This design system uses a high-contrast palette to ensure legibility and trust.
- **Primary (Indigo-900):** Used for core branding, primary actions, and navigational headers. It represents the "bedrock" of the platform.
- **Secondary (Teal-600):** Reserved for "Success" states, positive balance indicators, and completed transaction paths.
- **Tertiary (Amber-600):** Specifically for "Pending" or "Warning" states, signaling a need for attention without the urgency of an error.
- **Neutrals:** A range of cool grays (from #F5F7FA to #111827) provides the scaffolding for dashboards and data tables. Backgrounds should favor the lightest neutral to keep the UI feeling airy and modern.

## Typography
Typography is the cornerstone of the system's legibility. We pair **Plus Jakarta Sans** for headings to provide a modern, slightly geometric character that feels welcoming, with **Inter** for all functional UI elements and body text. 

Inter’s high x-height and neutral tone make it ideal for data-heavy tables and micro-copy. Use `label-md` for button text and `label-sm` for table headers (always uppercase). For numerical data in dashboards, ensure `tabular-nums` (monospaced numbers) is enabled in the CSS font-feature settings to maintain vertical alignment in columns.

## Layout & Spacing
The layout follows a strict **12-column fluid grid** for desktop and a **4-column grid** for mobile. 

- **Desktop (1024px+):** 24px gutters with 40px side margins. 
- **Tablet (768px - 1023px):** 20px gutters with 24px side margins.
- **Mobile (<768px):** 16px gutters with 16px side margins.

Horizontal spacing is governed by an 8px scale. For vertical rhythm, use "stack" variables to ensure consistent separation between card elements and sections. Dashboard layouts should utilize a fixed-width left navigation (240px) while the main content area remains fluid up to a maximum width of 1280px to prevent line lengths from becoming illegible.

## Elevation & Depth
This design system uses **Tonal Layers** supplemented by **Ambient Shadows** to create a sense of organized hierarchy. 

- **Level 0 (Base):** The main background using the neutral off-white.
- **Level 1 (Cards/Surface):** Pure white background with a very soft, diffused shadow (0px 2px 4px rgba(0,0,0,0.05)). This is where 90% of content resides.
- **Level 2 (Dropdowns/Modals):** Pure white with a more pronounced shadow (0px 10px 25px rgba(0,0,0,0.1)) and a 1px subtle gray border (#E2E8F0).

Avoid heavy black shadows. Instead, use shadows tinted with the primary navy color at very low opacities (2-5%) to maintain a clean, high-end fintech aesthetic.

## Shapes
The shape language is "Semi-Rounded." 

A standard border-radius of **8px (0.5rem)** is applied to buttons, input fields, and small UI components. Larger containers like dashboard cards and modals should use **16px (1rem)** for a softer, more modern appearance. Full-circle pill shapes are reserved exclusively for status "badges" (e.g., "Paid", "Refunded") to distinguish them from interactive buttons.

## Components
- **Buttons:** Primary buttons use the Navy background with white text. Secondary buttons use a 1px Navy border with Navy text. Hover states should involve a subtle shift to a slightly lighter tint.
- **Input Fields:** Use 8px rounded corners, a 1px light gray border, and 12px horizontal padding. On focus, the border should change to the Primary Navy with a 2px outer "glow" of 10% opacity Navy.
- **Status Chips:** Small, pill-shaped indicators. "Success" uses a light Teal background with dark Teal text; "Pending" uses light Amber background with dark Amber text.
- **Data Tables:** Headers should be `label-sm` in all-caps. Rows should have a subtle bottom border (#F1F5F9). Alternate row striping is optional, but hover highlights on rows are required for tracking data across columns.
- **Cards:** Always white background, 16px border-radius, and Level 1 shadow. Ensure 24px internal padding for content.
- **Navigation:** Vertical sidebar for desktop apps with high-contrast icons and active states marked by a 4px primary-color vertical bar on the left edge.