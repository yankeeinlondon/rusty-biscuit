//! Build script for biscuit-terminal
//!
//! Generates Tailwind v4 color definitions with OKLCH to sRGB conversion.
//! The generated file provides `impl Tailwind { pub const fn to_hdr_color(self) -> HdrColor }`
//! mapping all 227 Tailwind variants to accurate RGB + OKLCH values.

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// OKLCH color specification (Lightness, Chroma, Hue)
struct OklchColor {
    /// Lightness (0.0 to 1.0)
    l: f64,
    /// Chroma (0.0 to ~0.4)
    c: f64,
    /// Hue (0.0 to 360.0 degrees)
    h: f64,
}

impl OklchColor {
    const fn new(l: f64, c: f64, h: f64) -> Self {
        Self { l, c, h }
    }

    /// Convert OKLCH to sRGB with gamut mapping.
    ///
    /// Algorithm:
    /// 1. OKLCH (Lightness, Chroma, Hue) -> OKLab (L, a, b) via polar->cartesian
    /// 2. OKLab -> Linear RGB via matrix multiplication
    /// 3. Linear RGB -> sRGB via gamma correction
    fn to_srgb(&self) -> (u8, u8, u8) {
        // Step 1: OKLCH -> OKLab
        let h_rad = self.h * std::f64::consts::PI / 180.0;
        let lab_l = self.l;
        let lab_a = self.c * h_rad.cos();
        let lab_b = self.c * h_rad.sin();

        // Step 2: OKLab -> Linear sRGB
        // First compute L', M', S' intermediate values from OKLab
        let l_prime = lab_l + 0.3963377774 * lab_a + 0.2158037573 * lab_b;
        let m_prime = lab_l - 0.1055613458 * lab_a - 0.0638541728 * lab_b;
        let s_prime = lab_l - 0.0894841775 * lab_a - 1.2914855480 * lab_b;

        // Cube each to get L, M, S
        let l_cubed = l_prime * l_prime * l_prime;
        let m_cubed = m_prime * m_prime * m_prime;
        let s_cubed = s_prime * s_prime * s_prime;

        // Apply OKLab -> Linear sRGB matrix
        let r_linear = 4.0767416621 * l_cubed - 3.3077115913 * m_cubed + 0.2309699292 * s_cubed;
        let g_linear = -1.2684380046 * l_cubed + 2.6097574011 * m_cubed - 0.3413193965 * s_cubed;
        let b_linear = -0.0041960863 * l_cubed - 0.7034186147 * m_cubed + 1.7076147010 * s_cubed;

        // Step 3: Linear sRGB -> sRGB via gamma correction
        fn gamma_correct(linear: f64) -> f64 {
            if linear <= 0.0031308 {
                12.92 * linear
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        }

        let r = gamma_correct(r_linear);
        let g = gamma_correct(g_linear);
        let b = gamma_correct(b_linear);

        // Clamp to 0-255 range and convert to u8
        fn to_u8(v: f64) -> u8 {
            (v.clamp(0.0, 1.0) * 255.0).round() as u8
        }

        (to_u8(r), to_u8(g), to_u8(b))
    }
}

/// Tailwind v4 color definition with variant name, OKLCH values, and ANSI fallback
struct TailwindColorDef {
    variant: &'static str,
    oklch: OklchColor,
    fallback: &'static str,
    css_var: &'static str,
}

/// All Tailwind v4 OKLCH color definitions from official source
const TAILWIND_COLORS: &[TailwindColorDef] = &[
    // Special colors
    TailwindColorDef {
        variant: "Black",
        oklch: OklchColor::new(0.0, 0.0, 0.0),
        fallback: "BasicColor::Black",
        css_var: "--color-black",
    },
    TailwindColorDef {
        variant: "White",
        oklch: OklchColor::new(1.0, 0.0, 0.0),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-white",
    },
    // Slate
    TailwindColorDef {
        variant: "Slate50",
        oklch: OklchColor::new(0.984, 0.003, 247.858),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-slate-50",
    },
    TailwindColorDef {
        variant: "Slate100",
        oklch: OklchColor::new(0.968, 0.007, 247.896),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-slate-100",
    },
    TailwindColorDef {
        variant: "Slate200",
        oklch: OklchColor::new(0.929, 0.013, 255.508),
        fallback: "BasicColor::White",
        css_var: "--color-slate-200",
    },
    TailwindColorDef {
        variant: "Slate300",
        oklch: OklchColor::new(0.869, 0.022, 252.894),
        fallback: "BasicColor::White",
        css_var: "--color-slate-300",
    },
    TailwindColorDef {
        variant: "Slate400",
        oklch: OklchColor::new(0.704, 0.04, 256.788),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-slate-400",
    },
    TailwindColorDef {
        variant: "Slate500",
        oklch: OklchColor::new(0.554, 0.046, 257.417),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-slate-500",
    },
    TailwindColorDef {
        variant: "Slate600",
        oklch: OklchColor::new(0.446, 0.043, 257.281),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-slate-600",
    },
    TailwindColorDef {
        variant: "Slate700",
        oklch: OklchColor::new(0.372, 0.044, 257.287),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-slate-700",
    },
    TailwindColorDef {
        variant: "Slate800",
        oklch: OklchColor::new(0.279, 0.041, 260.031),
        fallback: "BasicColor::Black",
        css_var: "--color-slate-800",
    },
    TailwindColorDef {
        variant: "Slate900",
        oklch: OklchColor::new(0.208, 0.042, 265.755),
        fallback: "BasicColor::Black",
        css_var: "--color-slate-900",
    },
    TailwindColorDef {
        variant: "Slate950",
        oklch: OklchColor::new(0.129, 0.042, 264.695),
        fallback: "BasicColor::Black",
        css_var: "--color-slate-950",
    },
    // Gray
    TailwindColorDef {
        variant: "Gray50",
        oklch: OklchColor::new(0.985, 0.002, 247.839),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-gray-50",
    },
    TailwindColorDef {
        variant: "Gray100",
        oklch: OklchColor::new(0.967, 0.003, 264.542),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-gray-100",
    },
    TailwindColorDef {
        variant: "Gray200",
        oklch: OklchColor::new(0.928, 0.006, 264.531),
        fallback: "BasicColor::White",
        css_var: "--color-gray-200",
    },
    TailwindColorDef {
        variant: "Gray300",
        oklch: OklchColor::new(0.872, 0.01, 258.338),
        fallback: "BasicColor::White",
        css_var: "--color-gray-300",
    },
    TailwindColorDef {
        variant: "Gray400",
        oklch: OklchColor::new(0.707, 0.022, 261.325),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-gray-400",
    },
    TailwindColorDef {
        variant: "Gray500",
        oklch: OklchColor::new(0.551, 0.027, 264.364),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-gray-500",
    },
    TailwindColorDef {
        variant: "Gray600",
        oklch: OklchColor::new(0.446, 0.03, 256.802),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-gray-600",
    },
    TailwindColorDef {
        variant: "Gray700",
        oklch: OklchColor::new(0.373, 0.034, 259.733),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-gray-700",
    },
    TailwindColorDef {
        variant: "Gray800",
        oklch: OklchColor::new(0.278, 0.033, 256.848),
        fallback: "BasicColor::Black",
        css_var: "--color-gray-800",
    },
    TailwindColorDef {
        variant: "Gray900",
        oklch: OklchColor::new(0.21, 0.034, 264.665),
        fallback: "BasicColor::Black",
        css_var: "--color-gray-900",
    },
    TailwindColorDef {
        variant: "Gray950",
        oklch: OklchColor::new(0.13, 0.028, 261.692),
        fallback: "BasicColor::Black",
        css_var: "--color-gray-950",
    },
    // Zinc
    TailwindColorDef {
        variant: "Zinc50",
        oklch: OklchColor::new(0.985, 0.0, 0.0),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-zinc-50",
    },
    TailwindColorDef {
        variant: "Zinc100",
        oklch: OklchColor::new(0.967, 0.001, 286.375),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-zinc-100",
    },
    TailwindColorDef {
        variant: "Zinc200",
        oklch: OklchColor::new(0.92, 0.004, 286.32),
        fallback: "BasicColor::White",
        css_var: "--color-zinc-200",
    },
    TailwindColorDef {
        variant: "Zinc300",
        oklch: OklchColor::new(0.871, 0.006, 286.286),
        fallback: "BasicColor::White",
        css_var: "--color-zinc-300",
    },
    TailwindColorDef {
        variant: "Zinc400",
        oklch: OklchColor::new(0.705, 0.015, 286.067),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-zinc-400",
    },
    TailwindColorDef {
        variant: "Zinc500",
        oklch: OklchColor::new(0.552, 0.016, 285.938),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-zinc-500",
    },
    TailwindColorDef {
        variant: "Zinc600",
        oklch: OklchColor::new(0.442, 0.017, 285.786),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-zinc-600",
    },
    TailwindColorDef {
        variant: "Zinc700",
        oklch: OklchColor::new(0.37, 0.013, 285.805),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-zinc-700",
    },
    TailwindColorDef {
        variant: "Zinc800",
        oklch: OklchColor::new(0.274, 0.006, 286.033),
        fallback: "BasicColor::Black",
        css_var: "--color-zinc-800",
    },
    TailwindColorDef {
        variant: "Zinc900",
        oklch: OklchColor::new(0.21, 0.006, 285.885),
        fallback: "BasicColor::Black",
        css_var: "--color-zinc-900",
    },
    TailwindColorDef {
        variant: "Zinc950",
        oklch: OklchColor::new(0.141, 0.005, 285.823),
        fallback: "BasicColor::Black",
        css_var: "--color-zinc-950",
    },
    // Neutral
    TailwindColorDef {
        variant: "Neutral50",
        oklch: OklchColor::new(0.985, 0.0, 0.0),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-neutral-50",
    },
    TailwindColorDef {
        variant: "Neutral100",
        oklch: OklchColor::new(0.97, 0.0, 0.0),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-neutral-100",
    },
    TailwindColorDef {
        variant: "Neutral200",
        oklch: OklchColor::new(0.922, 0.0, 0.0),
        fallback: "BasicColor::White",
        css_var: "--color-neutral-200",
    },
    TailwindColorDef {
        variant: "Neutral300",
        oklch: OklchColor::new(0.87, 0.0, 0.0),
        fallback: "BasicColor::White",
        css_var: "--color-neutral-300",
    },
    TailwindColorDef {
        variant: "Neutral400",
        oklch: OklchColor::new(0.708, 0.0, 0.0),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-neutral-400",
    },
    TailwindColorDef {
        variant: "Neutral500",
        oklch: OklchColor::new(0.556, 0.0, 0.0),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-neutral-500",
    },
    TailwindColorDef {
        variant: "Neutral600",
        oklch: OklchColor::new(0.439, 0.0, 0.0),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-neutral-600",
    },
    TailwindColorDef {
        variant: "Neutral700",
        oklch: OklchColor::new(0.371, 0.0, 0.0),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-neutral-700",
    },
    TailwindColorDef {
        variant: "Neutral800",
        oklch: OklchColor::new(0.269, 0.0, 0.0),
        fallback: "BasicColor::Black",
        css_var: "--color-neutral-800",
    },
    TailwindColorDef {
        variant: "Neutral900",
        oklch: OklchColor::new(0.205, 0.0, 0.0),
        fallback: "BasicColor::Black",
        css_var: "--color-neutral-900",
    },
    TailwindColorDef {
        variant: "Neutral950",
        oklch: OklchColor::new(0.145, 0.0, 0.0),
        fallback: "BasicColor::Black",
        css_var: "--color-neutral-950",
    },
    // Stone
    TailwindColorDef {
        variant: "Stone50",
        oklch: OklchColor::new(0.985, 0.001, 106.423),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-stone-50",
    },
    TailwindColorDef {
        variant: "Stone100",
        oklch: OklchColor::new(0.97, 0.001, 106.424),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-stone-100",
    },
    TailwindColorDef {
        variant: "Stone200",
        oklch: OklchColor::new(0.923, 0.003, 48.717),
        fallback: "BasicColor::White",
        css_var: "--color-stone-200",
    },
    TailwindColorDef {
        variant: "Stone300",
        oklch: OklchColor::new(0.869, 0.005, 56.366),
        fallback: "BasicColor::White",
        css_var: "--color-stone-300",
    },
    TailwindColorDef {
        variant: "Stone400",
        oklch: OklchColor::new(0.709, 0.01, 56.259),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-stone-400",
    },
    TailwindColorDef {
        variant: "Stone500",
        oklch: OklchColor::new(0.553, 0.013, 58.071),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-stone-500",
    },
    TailwindColorDef {
        variant: "Stone600",
        oklch: OklchColor::new(0.444, 0.011, 73.639),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-stone-600",
    },
    TailwindColorDef {
        variant: "Stone700",
        oklch: OklchColor::new(0.374, 0.01, 67.558),
        fallback: "BasicColor::BrightBlack",
        css_var: "--color-stone-700",
    },
    TailwindColorDef {
        variant: "Stone800",
        oklch: OklchColor::new(0.268, 0.007, 34.298),
        fallback: "BasicColor::Black",
        css_var: "--color-stone-800",
    },
    TailwindColorDef {
        variant: "Stone900",
        oklch: OklchColor::new(0.216, 0.006, 56.043),
        fallback: "BasicColor::Black",
        css_var: "--color-stone-900",
    },
    TailwindColorDef {
        variant: "Stone950",
        oklch: OklchColor::new(0.147, 0.004, 49.25),
        fallback: "BasicColor::Black",
        css_var: "--color-stone-950",
    },
    // Red
    TailwindColorDef {
        variant: "Red50",
        oklch: OklchColor::new(0.971, 0.013, 17.38),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-red-50",
    },
    TailwindColorDef {
        variant: "Red100",
        oklch: OklchColor::new(0.936, 0.032, 17.717),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-red-100",
    },
    TailwindColorDef {
        variant: "Red200",
        oklch: OklchColor::new(0.885, 0.062, 18.334),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-red-200",
    },
    TailwindColorDef {
        variant: "Red300",
        oklch: OklchColor::new(0.808, 0.114, 19.571),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-red-300",
    },
    TailwindColorDef {
        variant: "Red400",
        oklch: OklchColor::new(0.704, 0.191, 22.216),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-red-400",
    },
    TailwindColorDef {
        variant: "Red500",
        oklch: OklchColor::new(0.637, 0.237, 25.331),
        fallback: "BasicColor::Red",
        css_var: "--color-red-500",
    },
    TailwindColorDef {
        variant: "Red600",
        oklch: OklchColor::new(0.577, 0.245, 27.325),
        fallback: "BasicColor::Red",
        css_var: "--color-red-600",
    },
    TailwindColorDef {
        variant: "Red700",
        oklch: OklchColor::new(0.505, 0.213, 27.518),
        fallback: "BasicColor::Red",
        css_var: "--color-red-700",
    },
    TailwindColorDef {
        variant: "Red800",
        oklch: OklchColor::new(0.444, 0.177, 26.899),
        fallback: "BasicColor::Red",
        css_var: "--color-red-800",
    },
    TailwindColorDef {
        variant: "Red900",
        oklch: OklchColor::new(0.396, 0.141, 25.723),
        fallback: "BasicColor::Red",
        css_var: "--color-red-900",
    },
    TailwindColorDef {
        variant: "Red950",
        oklch: OklchColor::new(0.258, 0.092, 26.042),
        fallback: "BasicColor::Red",
        css_var: "--color-red-950",
    },
    // Orange
    TailwindColorDef {
        variant: "Orange50",
        oklch: OklchColor::new(0.98, 0.016, 73.684),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-orange-50",
    },
    TailwindColorDef {
        variant: "Orange100",
        oklch: OklchColor::new(0.954, 0.038, 75.164),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-orange-100",
    },
    TailwindColorDef {
        variant: "Orange200",
        oklch: OklchColor::new(0.901, 0.076, 70.697),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-orange-200",
    },
    TailwindColorDef {
        variant: "Orange300",
        oklch: OklchColor::new(0.837, 0.128, 66.29),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-orange-300",
    },
    TailwindColorDef {
        variant: "Orange400",
        oklch: OklchColor::new(0.75, 0.183, 55.934),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-orange-400",
    },
    TailwindColorDef {
        variant: "Orange500",
        oklch: OklchColor::new(0.705, 0.213, 47.604),
        fallback: "BasicColor::Yellow",
        css_var: "--color-orange-500",
    },
    TailwindColorDef {
        variant: "Orange600",
        oklch: OklchColor::new(0.646, 0.222, 41.116),
        fallback: "BasicColor::Yellow",
        css_var: "--color-orange-600",
    },
    TailwindColorDef {
        variant: "Orange700",
        oklch: OklchColor::new(0.553, 0.195, 38.402),
        fallback: "BasicColor::Yellow",
        css_var: "--color-orange-700",
    },
    TailwindColorDef {
        variant: "Orange800",
        oklch: OklchColor::new(0.47, 0.157, 37.304),
        fallback: "BasicColor::Yellow",
        css_var: "--color-orange-800",
    },
    TailwindColorDef {
        variant: "Orange900",
        oklch: OklchColor::new(0.408, 0.123, 38.172),
        fallback: "BasicColor::Yellow",
        css_var: "--color-orange-900",
    },
    TailwindColorDef {
        variant: "Orange950",
        oklch: OklchColor::new(0.266, 0.079, 36.259),
        fallback: "BasicColor::Red",
        css_var: "--color-orange-950",
    },
    // Amber
    TailwindColorDef {
        variant: "Amber50",
        oklch: OklchColor::new(0.987, 0.022, 95.277),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-amber-50",
    },
    TailwindColorDef {
        variant: "Amber100",
        oklch: OklchColor::new(0.962, 0.059, 95.617),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-amber-100",
    },
    TailwindColorDef {
        variant: "Amber200",
        oklch: OklchColor::new(0.924, 0.12, 95.746),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-amber-200",
    },
    TailwindColorDef {
        variant: "Amber300",
        oklch: OklchColor::new(0.879, 0.169, 91.605),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-amber-300",
    },
    TailwindColorDef {
        variant: "Amber400",
        oklch: OklchColor::new(0.828, 0.189, 84.429),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-amber-400",
    },
    TailwindColorDef {
        variant: "Amber500",
        oklch: OklchColor::new(0.769, 0.188, 70.08),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-500",
    },
    TailwindColorDef {
        variant: "Amber600",
        oklch: OklchColor::new(0.666, 0.179, 58.318),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-600",
    },
    TailwindColorDef {
        variant: "Amber700",
        oklch: OklchColor::new(0.555, 0.163, 48.998),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-700",
    },
    TailwindColorDef {
        variant: "Amber800",
        oklch: OklchColor::new(0.473, 0.137, 46.201),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-800",
    },
    TailwindColorDef {
        variant: "Amber900",
        oklch: OklchColor::new(0.414, 0.112, 45.904),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-900",
    },
    TailwindColorDef {
        variant: "Amber950",
        oklch: OklchColor::new(0.279, 0.077, 45.635),
        fallback: "BasicColor::Yellow",
        css_var: "--color-amber-950",
    },
    // Yellow
    TailwindColorDef {
        variant: "Yellow50",
        oklch: OklchColor::new(0.987, 0.026, 102.212),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-yellow-50",
    },
    TailwindColorDef {
        variant: "Yellow100",
        oklch: OklchColor::new(0.973, 0.071, 103.193),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-yellow-100",
    },
    TailwindColorDef {
        variant: "Yellow200",
        oklch: OklchColor::new(0.945, 0.129, 101.54),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-yellow-200",
    },
    TailwindColorDef {
        variant: "Yellow300",
        oklch: OklchColor::new(0.905, 0.182, 98.111),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-yellow-300",
    },
    TailwindColorDef {
        variant: "Yellow400",
        oklch: OklchColor::new(0.852, 0.199, 91.936),
        fallback: "BasicColor::BrightYellow",
        css_var: "--color-yellow-400",
    },
    TailwindColorDef {
        variant: "Yellow500",
        oklch: OklchColor::new(0.795, 0.184, 86.047),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-500",
    },
    TailwindColorDef {
        variant: "Yellow600",
        oklch: OklchColor::new(0.681, 0.162, 75.834),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-600",
    },
    TailwindColorDef {
        variant: "Yellow700",
        oklch: OklchColor::new(0.554, 0.135, 66.442),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-700",
    },
    TailwindColorDef {
        variant: "Yellow800",
        oklch: OklchColor::new(0.476, 0.114, 61.907),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-800",
    },
    TailwindColorDef {
        variant: "Yellow900",
        oklch: OklchColor::new(0.421, 0.095, 57.708),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-900",
    },
    TailwindColorDef {
        variant: "Yellow950",
        oklch: OklchColor::new(0.286, 0.066, 53.813),
        fallback: "BasicColor::Yellow",
        css_var: "--color-yellow-950",
    },
    // Lime
    TailwindColorDef {
        variant: "Lime50",
        oklch: OklchColor::new(0.986, 0.031, 120.757),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-lime-50",
    },
    TailwindColorDef {
        variant: "Lime100",
        oklch: OklchColor::new(0.967, 0.067, 122.328),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-lime-100",
    },
    TailwindColorDef {
        variant: "Lime200",
        oklch: OklchColor::new(0.938, 0.127, 124.321),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-lime-200",
    },
    TailwindColorDef {
        variant: "Lime300",
        oklch: OklchColor::new(0.897, 0.196, 126.665),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-lime-300",
    },
    TailwindColorDef {
        variant: "Lime400",
        oklch: OklchColor::new(0.841, 0.238, 128.85),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-lime-400",
    },
    TailwindColorDef {
        variant: "Lime500",
        oklch: OklchColor::new(0.768, 0.233, 130.85),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-500",
    },
    TailwindColorDef {
        variant: "Lime600",
        oklch: OklchColor::new(0.648, 0.2, 131.684),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-600",
    },
    TailwindColorDef {
        variant: "Lime700",
        oklch: OklchColor::new(0.532, 0.157, 131.589),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-700",
    },
    TailwindColorDef {
        variant: "Lime800",
        oklch: OklchColor::new(0.453, 0.124, 130.933),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-800",
    },
    TailwindColorDef {
        variant: "Lime900",
        oklch: OklchColor::new(0.405, 0.101, 131.063),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-900",
    },
    TailwindColorDef {
        variant: "Lime950",
        oklch: OklchColor::new(0.274, 0.072, 132.109),
        fallback: "BasicColor::Green",
        css_var: "--color-lime-950",
    },
    // Green
    TailwindColorDef {
        variant: "Green50",
        oklch: OklchColor::new(0.982, 0.018, 155.826),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-green-50",
    },
    TailwindColorDef {
        variant: "Green100",
        oklch: OklchColor::new(0.962, 0.044, 156.743),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-green-100",
    },
    TailwindColorDef {
        variant: "Green200",
        oklch: OklchColor::new(0.925, 0.084, 155.995),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-green-200",
    },
    TailwindColorDef {
        variant: "Green300",
        oklch: OklchColor::new(0.871, 0.15, 154.449),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-green-300",
    },
    TailwindColorDef {
        variant: "Green400",
        oklch: OklchColor::new(0.792, 0.209, 151.711),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-green-400",
    },
    TailwindColorDef {
        variant: "Green500",
        oklch: OklchColor::new(0.723, 0.219, 149.579),
        fallback: "BasicColor::Green",
        css_var: "--color-green-500",
    },
    TailwindColorDef {
        variant: "Green600",
        oklch: OklchColor::new(0.627, 0.194, 149.214),
        fallback: "BasicColor::Green",
        css_var: "--color-green-600",
    },
    TailwindColorDef {
        variant: "Green700",
        oklch: OklchColor::new(0.527, 0.154, 150.069),
        fallback: "BasicColor::Green",
        css_var: "--color-green-700",
    },
    TailwindColorDef {
        variant: "Green800",
        oklch: OklchColor::new(0.448, 0.119, 151.328),
        fallback: "BasicColor::Green",
        css_var: "--color-green-800",
    },
    TailwindColorDef {
        variant: "Green900",
        oklch: OklchColor::new(0.393, 0.095, 152.535),
        fallback: "BasicColor::Green",
        css_var: "--color-green-900",
    },
    TailwindColorDef {
        variant: "Green950",
        oklch: OklchColor::new(0.266, 0.065, 152.934),
        fallback: "BasicColor::Green",
        css_var: "--color-green-950",
    },
    // Emerald
    TailwindColorDef {
        variant: "Emerald50",
        oklch: OklchColor::new(0.979, 0.021, 166.113),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-emerald-50",
    },
    TailwindColorDef {
        variant: "Emerald100",
        oklch: OklchColor::new(0.95, 0.052, 163.051),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-emerald-100",
    },
    TailwindColorDef {
        variant: "Emerald200",
        oklch: OklchColor::new(0.905, 0.093, 164.15),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-emerald-200",
    },
    TailwindColorDef {
        variant: "Emerald300",
        oklch: OklchColor::new(0.845, 0.143, 164.978),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-emerald-300",
    },
    TailwindColorDef {
        variant: "Emerald400",
        oklch: OklchColor::new(0.765, 0.177, 163.223),
        fallback: "BasicColor::BrightGreen",
        css_var: "--color-emerald-400",
    },
    TailwindColorDef {
        variant: "Emerald500",
        oklch: OklchColor::new(0.696, 0.17, 162.48),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-500",
    },
    TailwindColorDef {
        variant: "Emerald600",
        oklch: OklchColor::new(0.596, 0.145, 163.225),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-600",
    },
    TailwindColorDef {
        variant: "Emerald700",
        oklch: OklchColor::new(0.508, 0.118, 165.612),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-700",
    },
    TailwindColorDef {
        variant: "Emerald800",
        oklch: OklchColor::new(0.432, 0.095, 166.913),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-800",
    },
    TailwindColorDef {
        variant: "Emerald900",
        oklch: OklchColor::new(0.378, 0.077, 168.94),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-900",
    },
    TailwindColorDef {
        variant: "Emerald950",
        oklch: OklchColor::new(0.262, 0.051, 172.552),
        fallback: "BasicColor::Green",
        css_var: "--color-emerald-950",
    },
    // Teal
    TailwindColorDef {
        variant: "Teal50",
        oklch: OklchColor::new(0.984, 0.014, 180.72),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-teal-50",
    },
    TailwindColorDef {
        variant: "Teal100",
        oklch: OklchColor::new(0.953, 0.051, 180.801),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-teal-100",
    },
    TailwindColorDef {
        variant: "Teal200",
        oklch: OklchColor::new(0.91, 0.096, 180.426),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-teal-200",
    },
    TailwindColorDef {
        variant: "Teal300",
        oklch: OklchColor::new(0.855, 0.138, 181.071),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-teal-300",
    },
    TailwindColorDef {
        variant: "Teal400",
        oklch: OklchColor::new(0.777, 0.152, 181.912),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-teal-400",
    },
    TailwindColorDef {
        variant: "Teal500",
        oklch: OklchColor::new(0.704, 0.14, 182.503),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-500",
    },
    TailwindColorDef {
        variant: "Teal600",
        oklch: OklchColor::new(0.6, 0.118, 184.704),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-600",
    },
    TailwindColorDef {
        variant: "Teal700",
        oklch: OklchColor::new(0.511, 0.096, 186.391),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-700",
    },
    TailwindColorDef {
        variant: "Teal800",
        oklch: OklchColor::new(0.437, 0.078, 188.216),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-800",
    },
    TailwindColorDef {
        variant: "Teal900",
        oklch: OklchColor::new(0.386, 0.063, 188.416),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-900",
    },
    TailwindColorDef {
        variant: "Teal950",
        oklch: OklchColor::new(0.277, 0.046, 192.524),
        fallback: "BasicColor::Cyan",
        css_var: "--color-teal-950",
    },
    // Cyan
    TailwindColorDef {
        variant: "Cyan50",
        oklch: OklchColor::new(0.984, 0.019, 200.873),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-cyan-50",
    },
    TailwindColorDef {
        variant: "Cyan100",
        oklch: OklchColor::new(0.956, 0.045, 203.388),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-cyan-100",
    },
    TailwindColorDef {
        variant: "Cyan200",
        oklch: OklchColor::new(0.917, 0.08, 205.041),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-cyan-200",
    },
    TailwindColorDef {
        variant: "Cyan300",
        oklch: OklchColor::new(0.865, 0.127, 207.078),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-cyan-300",
    },
    TailwindColorDef {
        variant: "Cyan400",
        oklch: OklchColor::new(0.789, 0.154, 211.53),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-cyan-400",
    },
    TailwindColorDef {
        variant: "Cyan500",
        oklch: OklchColor::new(0.715, 0.143, 215.221),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-500",
    },
    TailwindColorDef {
        variant: "Cyan600",
        oklch: OklchColor::new(0.609, 0.126, 221.723),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-600",
    },
    TailwindColorDef {
        variant: "Cyan700",
        oklch: OklchColor::new(0.52, 0.105, 223.128),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-700",
    },
    TailwindColorDef {
        variant: "Cyan800",
        oklch: OklchColor::new(0.45, 0.085, 224.283),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-800",
    },
    TailwindColorDef {
        variant: "Cyan900",
        oklch: OklchColor::new(0.398, 0.07, 227.392),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-900",
    },
    TailwindColorDef {
        variant: "Cyan950",
        oklch: OklchColor::new(0.302, 0.056, 229.695),
        fallback: "BasicColor::Cyan",
        css_var: "--color-cyan-950",
    },
    // Sky
    TailwindColorDef {
        variant: "Sky50",
        oklch: OklchColor::new(0.977, 0.013, 236.62),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-sky-50",
    },
    TailwindColorDef {
        variant: "Sky100",
        oklch: OklchColor::new(0.951, 0.026, 236.824),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-sky-100",
    },
    TailwindColorDef {
        variant: "Sky200",
        oklch: OklchColor::new(0.901, 0.058, 230.902),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-sky-200",
    },
    TailwindColorDef {
        variant: "Sky300",
        oklch: OklchColor::new(0.828, 0.111, 230.318),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-sky-300",
    },
    TailwindColorDef {
        variant: "Sky400",
        oklch: OklchColor::new(0.746, 0.16, 232.661),
        fallback: "BasicColor::BrightCyan",
        css_var: "--color-sky-400",
    },
    TailwindColorDef {
        variant: "Sky500",
        oklch: OklchColor::new(0.685, 0.169, 237.323),
        fallback: "BasicColor::Cyan",
        css_var: "--color-sky-500",
    },
    TailwindColorDef {
        variant: "Sky600",
        oklch: OklchColor::new(0.588, 0.158, 241.966),
        fallback: "BasicColor::Cyan",
        css_var: "--color-sky-600",
    },
    TailwindColorDef {
        variant: "Sky700",
        oklch: OklchColor::new(0.5, 0.134, 242.749),
        fallback: "BasicColor::Cyan",
        css_var: "--color-sky-700",
    },
    TailwindColorDef {
        variant: "Sky800",
        oklch: OklchColor::new(0.443, 0.11, 240.79),
        fallback: "BasicColor::Cyan",
        css_var: "--color-sky-800",
    },
    TailwindColorDef {
        variant: "Sky900",
        oklch: OklchColor::new(0.391, 0.09, 240.876),
        fallback: "BasicColor::Blue",
        css_var: "--color-sky-900",
    },
    TailwindColorDef {
        variant: "Sky950",
        oklch: OklchColor::new(0.293, 0.066, 243.157),
        fallback: "BasicColor::Blue",
        css_var: "--color-sky-950",
    },
    // Blue
    TailwindColorDef {
        variant: "Blue50",
        oklch: OklchColor::new(0.97, 0.014, 254.604),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-blue-50",
    },
    TailwindColorDef {
        variant: "Blue100",
        oklch: OklchColor::new(0.932, 0.032, 255.585),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-blue-100",
    },
    TailwindColorDef {
        variant: "Blue200",
        oklch: OklchColor::new(0.882, 0.059, 254.128),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-blue-200",
    },
    TailwindColorDef {
        variant: "Blue300",
        oklch: OklchColor::new(0.809, 0.105, 251.813),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-blue-300",
    },
    TailwindColorDef {
        variant: "Blue400",
        oklch: OklchColor::new(0.707, 0.165, 254.624),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-blue-400",
    },
    TailwindColorDef {
        variant: "Blue500",
        oklch: OklchColor::new(0.623, 0.214, 259.815),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-500",
    },
    TailwindColorDef {
        variant: "Blue600",
        oklch: OklchColor::new(0.546, 0.245, 262.881),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-600",
    },
    TailwindColorDef {
        variant: "Blue700",
        oklch: OklchColor::new(0.488, 0.243, 264.376),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-700",
    },
    TailwindColorDef {
        variant: "Blue800",
        oklch: OklchColor::new(0.424, 0.199, 265.638),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-800",
    },
    TailwindColorDef {
        variant: "Blue900",
        oklch: OklchColor::new(0.379, 0.146, 265.522),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-900",
    },
    TailwindColorDef {
        variant: "Blue950",
        oklch: OklchColor::new(0.282, 0.091, 267.935),
        fallback: "BasicColor::Blue",
        css_var: "--color-blue-950",
    },
    // Indigo
    TailwindColorDef {
        variant: "Indigo50",
        oklch: OklchColor::new(0.962, 0.018, 272.314),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-indigo-50",
    },
    TailwindColorDef {
        variant: "Indigo100",
        oklch: OklchColor::new(0.93, 0.034, 272.788),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-indigo-100",
    },
    TailwindColorDef {
        variant: "Indigo200",
        oklch: OklchColor::new(0.87, 0.065, 274.039),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-indigo-200",
    },
    TailwindColorDef {
        variant: "Indigo300",
        oklch: OklchColor::new(0.785, 0.115, 274.713),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-indigo-300",
    },
    TailwindColorDef {
        variant: "Indigo400",
        oklch: OklchColor::new(0.673, 0.182, 276.935),
        fallback: "BasicColor::BrightBlue",
        css_var: "--color-indigo-400",
    },
    TailwindColorDef {
        variant: "Indigo500",
        oklch: OklchColor::new(0.585, 0.233, 277.117),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-500",
    },
    TailwindColorDef {
        variant: "Indigo600",
        oklch: OklchColor::new(0.511, 0.262, 276.966),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-600",
    },
    TailwindColorDef {
        variant: "Indigo700",
        oklch: OklchColor::new(0.457, 0.24, 277.023),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-700",
    },
    TailwindColorDef {
        variant: "Indigo800",
        oklch: OklchColor::new(0.398, 0.195, 277.366),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-800",
    },
    TailwindColorDef {
        variant: "Indigo900",
        oklch: OklchColor::new(0.359, 0.144, 278.697),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-900",
    },
    TailwindColorDef {
        variant: "Indigo950",
        oklch: OklchColor::new(0.257, 0.09, 281.288),
        fallback: "BasicColor::Blue",
        css_var: "--color-indigo-950",
    },
    // Violet
    TailwindColorDef {
        variant: "Violet50",
        oklch: OklchColor::new(0.969, 0.016, 293.756),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-violet-50",
    },
    TailwindColorDef {
        variant: "Violet100",
        oklch: OklchColor::new(0.943, 0.029, 294.588),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-violet-100",
    },
    TailwindColorDef {
        variant: "Violet200",
        oklch: OklchColor::new(0.894, 0.057, 293.283),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-violet-200",
    },
    TailwindColorDef {
        variant: "Violet300",
        oklch: OklchColor::new(0.811, 0.111, 293.571),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-violet-300",
    },
    TailwindColorDef {
        variant: "Violet400",
        oklch: OklchColor::new(0.702, 0.183, 293.541),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-violet-400",
    },
    TailwindColorDef {
        variant: "Violet500",
        oklch: OklchColor::new(0.606, 0.25, 292.717),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-500",
    },
    TailwindColorDef {
        variant: "Violet600",
        oklch: OklchColor::new(0.541, 0.281, 293.009),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-600",
    },
    TailwindColorDef {
        variant: "Violet700",
        oklch: OklchColor::new(0.491, 0.27, 292.581),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-700",
    },
    TailwindColorDef {
        variant: "Violet800",
        oklch: OklchColor::new(0.432, 0.232, 292.759),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-800",
    },
    TailwindColorDef {
        variant: "Violet900",
        oklch: OklchColor::new(0.38, 0.189, 293.745),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-900",
    },
    TailwindColorDef {
        variant: "Violet950",
        oklch: OklchColor::new(0.283, 0.141, 291.089),
        fallback: "BasicColor::Magenta",
        css_var: "--color-violet-950",
    },
    // Purple
    TailwindColorDef {
        variant: "Purple50",
        oklch: OklchColor::new(0.977, 0.014, 308.299),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-purple-50",
    },
    TailwindColorDef {
        variant: "Purple100",
        oklch: OklchColor::new(0.946, 0.033, 307.174),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-purple-100",
    },
    TailwindColorDef {
        variant: "Purple200",
        oklch: OklchColor::new(0.902, 0.063, 306.703),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-purple-200",
    },
    TailwindColorDef {
        variant: "Purple300",
        oklch: OklchColor::new(0.827, 0.119, 306.383),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-purple-300",
    },
    TailwindColorDef {
        variant: "Purple400",
        oklch: OklchColor::new(0.714, 0.203, 305.504),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-purple-400",
    },
    TailwindColorDef {
        variant: "Purple500",
        oklch: OklchColor::new(0.627, 0.265, 303.9),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-500",
    },
    TailwindColorDef {
        variant: "Purple600",
        oklch: OklchColor::new(0.558, 0.288, 302.321),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-600",
    },
    TailwindColorDef {
        variant: "Purple700",
        oklch: OklchColor::new(0.496, 0.265, 301.924),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-700",
    },
    TailwindColorDef {
        variant: "Purple800",
        oklch: OklchColor::new(0.438, 0.218, 303.724),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-800",
    },
    TailwindColorDef {
        variant: "Purple900",
        oklch: OklchColor::new(0.381, 0.176, 304.987),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-900",
    },
    TailwindColorDef {
        variant: "Purple950",
        oklch: OklchColor::new(0.291, 0.149, 302.717),
        fallback: "BasicColor::Magenta",
        css_var: "--color-purple-950",
    },
    // Fuchsia
    TailwindColorDef {
        variant: "Fuchsia50",
        oklch: OklchColor::new(0.977, 0.017, 320.058),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-fuchsia-50",
    },
    TailwindColorDef {
        variant: "Fuchsia100",
        oklch: OklchColor::new(0.952, 0.037, 318.852),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-fuchsia-100",
    },
    TailwindColorDef {
        variant: "Fuchsia200",
        oklch: OklchColor::new(0.903, 0.076, 319.62),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-fuchsia-200",
    },
    TailwindColorDef {
        variant: "Fuchsia300",
        oklch: OklchColor::new(0.833, 0.145, 321.434),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-fuchsia-300",
    },
    TailwindColorDef {
        variant: "Fuchsia400",
        oklch: OklchColor::new(0.74, 0.238, 322.16),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-fuchsia-400",
    },
    TailwindColorDef {
        variant: "Fuchsia500",
        oklch: OklchColor::new(0.667, 0.295, 322.15),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-500",
    },
    TailwindColorDef {
        variant: "Fuchsia600",
        oklch: OklchColor::new(0.591, 0.293, 322.896),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-600",
    },
    TailwindColorDef {
        variant: "Fuchsia700",
        oklch: OklchColor::new(0.518, 0.253, 323.949),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-700",
    },
    TailwindColorDef {
        variant: "Fuchsia800",
        oklch: OklchColor::new(0.452, 0.211, 324.591),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-800",
    },
    TailwindColorDef {
        variant: "Fuchsia900",
        oklch: OklchColor::new(0.401, 0.17, 325.612),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-900",
    },
    TailwindColorDef {
        variant: "Fuchsia950",
        oklch: OklchColor::new(0.293, 0.136, 325.661),
        fallback: "BasicColor::Magenta",
        css_var: "--color-fuchsia-950",
    },
    // Pink
    TailwindColorDef {
        variant: "Pink50",
        oklch: OklchColor::new(0.971, 0.014, 343.198),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-pink-50",
    },
    TailwindColorDef {
        variant: "Pink100",
        oklch: OklchColor::new(0.948, 0.028, 342.258),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-pink-100",
    },
    TailwindColorDef {
        variant: "Pink200",
        oklch: OklchColor::new(0.899, 0.061, 343.231),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-pink-200",
    },
    TailwindColorDef {
        variant: "Pink300",
        oklch: OklchColor::new(0.823, 0.12, 346.018),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-pink-300",
    },
    TailwindColorDef {
        variant: "Pink400",
        oklch: OklchColor::new(0.718, 0.202, 349.761),
        fallback: "BasicColor::BrightMagenta",
        css_var: "--color-pink-400",
    },
    TailwindColorDef {
        variant: "Pink500",
        oklch: OklchColor::new(0.656, 0.241, 354.308),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-500",
    },
    TailwindColorDef {
        variant: "Pink600",
        oklch: OklchColor::new(0.592, 0.249, 0.584),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-600",
    },
    TailwindColorDef {
        variant: "Pink700",
        oklch: OklchColor::new(0.525, 0.223, 3.958),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-700",
    },
    TailwindColorDef {
        variant: "Pink800",
        oklch: OklchColor::new(0.459, 0.187, 3.815),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-800",
    },
    TailwindColorDef {
        variant: "Pink900",
        oklch: OklchColor::new(0.408, 0.153, 2.432),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-900",
    },
    TailwindColorDef {
        variant: "Pink950",
        oklch: OklchColor::new(0.284, 0.109, 3.907),
        fallback: "BasicColor::Magenta",
        css_var: "--color-pink-950",
    },
    // Rose
    TailwindColorDef {
        variant: "Rose50",
        oklch: OklchColor::new(0.969, 0.015, 12.422),
        fallback: "BasicColor::BrightWhite",
        css_var: "--color-rose-50",
    },
    TailwindColorDef {
        variant: "Rose100",
        oklch: OklchColor::new(0.941, 0.03, 12.58),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-rose-100",
    },
    TailwindColorDef {
        variant: "Rose200",
        oklch: OklchColor::new(0.892, 0.058, 10.001),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-rose-200",
    },
    TailwindColorDef {
        variant: "Rose300",
        oklch: OklchColor::new(0.81, 0.117, 11.638),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-rose-300",
    },
    TailwindColorDef {
        variant: "Rose400",
        oklch: OklchColor::new(0.712, 0.194, 13.428),
        fallback: "BasicColor::BrightRed",
        css_var: "--color-rose-400",
    },
    TailwindColorDef {
        variant: "Rose500",
        oklch: OklchColor::new(0.645, 0.246, 16.439),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-500",
    },
    TailwindColorDef {
        variant: "Rose600",
        oklch: OklchColor::new(0.586, 0.253, 17.585),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-600",
    },
    TailwindColorDef {
        variant: "Rose700",
        oklch: OklchColor::new(0.514, 0.222, 16.935),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-700",
    },
    TailwindColorDef {
        variant: "Rose800",
        oklch: OklchColor::new(0.455, 0.188, 13.697),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-800",
    },
    TailwindColorDef {
        variant: "Rose900",
        oklch: OklchColor::new(0.41, 0.159, 10.272),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-900",
    },
    TailwindColorDef {
        variant: "Rose950",
        oklch: OklchColor::new(0.271, 0.105, 12.094),
        fallback: "BasicColor::Red",
        css_var: "--color-rose-950",
    },
];

fn main() {
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("tailwind_colors.rs");
    let file = File::create(&dest_path).expect("Failed to create tailwind_colors.rs");
    let mut writer = BufWriter::new(file);

    // Write header
    // Note: This file is included in utils/color.rs, so imports use self:: prefix
    writeln!(writer, "// Auto-generated by build.rs - DO NOT EDIT").unwrap();
    writeln!(writer, "// Tailwind v4 OKLCH color definitions").unwrap();
    writeln!(writer).unwrap();

    // Write impl block for to_hdr_color
    writeln!(writer, "impl Tailwind {{").unwrap();
    writeln!(
        writer,
        "    /// Converts this Tailwind color variant to an `HdrColor` with RGB and OKLCH values."
    )
    .unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(
        writer,
        "    /// RGB values are computed from Tailwind v4's OKLCH definitions using"
    )
    .unwrap();
    writeln!(
        writer,
        "    /// the standard OKLCH->OKLab->Linear sRGB->sRGB conversion pipeline."
    )
    .unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(writer, "    /// ## Returns").unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(
        writer,
        "    /// `None` for special values (Inherit, Current, Transparent),"
    )
    .unwrap();
    writeln!(
        writer,
        "    /// `Some(HdrColor)` for all concrete color variants."
    )
    .unwrap();
    writeln!(
        writer,
        "    pub const fn to_hdr_color(self) -> Option<HdrColor> {{"
    )
    .unwrap();
    writeln!(writer, "        match self {{").unwrap();

    // Special cases
    writeln!(writer, "            Tailwind::Inherit => None,").unwrap();
    writeln!(writer, "            Tailwind::Current => None,").unwrap();
    writeln!(writer, "            Tailwind::Transparent => None,").unwrap();

    // Generate match arms for all colors
    for color in TAILWIND_COLORS {
        let (r, g, b) = color.oklch.to_srgb();
        writeln!(
            writer,
            "            Tailwind::{} => Some(HdrColor::new({}, {}, {}, {}, {}f32, {}f32, {}f32)),",
            color.variant, r, g, b, color.fallback, color.oklch.l, color.oklch.c, color.oklch.h
        )
        .unwrap();
    }

    writeln!(writer, "        }}").unwrap();
    writeln!(writer, "    }}").unwrap();
    writeln!(writer).unwrap();

    // Write css_var method
    writeln!(
        writer,
        "    /// Returns the CSS variable name for this Tailwind color."
    )
    .unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(writer, "    /// For use with `var(--color-name)` in CSS.").unwrap();
    writeln!(writer, "    pub const fn css_var(self) -> &'static str {{").unwrap();
    writeln!(writer, "        match self {{").unwrap();
    writeln!(writer, "            Tailwind::Inherit => \"inherit\",").unwrap();
    writeln!(writer, "            Tailwind::Current => \"currentColor\",").unwrap();
    writeln!(
        writer,
        "            Tailwind::Transparent => \"transparent\","
    )
    .unwrap();

    for color in TAILWIND_COLORS {
        writeln!(
            writer,
            "            Tailwind::{} => \"{}\",",
            color.variant, color.css_var
        )
        .unwrap();
    }

    writeln!(writer, "        }}").unwrap();
    writeln!(writer, "    }}").unwrap();
    writeln!(writer).unwrap();

    // Write hex method
    writeln!(
        writer,
        "    /// Returns the hex color code for this Tailwind color."
    )
    .unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(
        writer,
        "    /// Returns `None` for special values (Inherit, Current, Transparent)."
    )
    .unwrap();
    writeln!(
        writer,
        "    pub const fn hex(self) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(writer, "        match self {{").unwrap();
    writeln!(writer, "            Tailwind::Inherit => None,").unwrap();
    writeln!(writer, "            Tailwind::Current => None,").unwrap();
    writeln!(writer, "            Tailwind::Transparent => None,").unwrap();

    for color in TAILWIND_COLORS {
        let (r, g, b) = color.oklch.to_srgb();
        writeln!(
            writer,
            "            Tailwind::{} => Some(\"#{:02x}{:02x}{:02x}\"),",
            color.variant, r, g, b
        )
        .unwrap();
    }

    writeln!(writer, "        }}").unwrap();
    writeln!(writer, "    }}").unwrap();

    // Write palette_defaults method
    writeln!(writer).unwrap();
    writeln!(writer, "    /// Every concrete Tailwind palette color as a").unwrap();
    writeln!(writer, "    /// `(custom-property-name, hex-value)` pair.").unwrap();
    writeln!(writer, "    ///").unwrap();
    writeln!(
        writer,
        "    /// Names omit the leading `--`. Special values (`Inherit`,"
    )
    .unwrap();
    writeln!(
        writer,
        "    /// `Current`, `Transparent`) are excluded — they have no hex value."
    )
    .unwrap();
    writeln!(
        writer,
        "    pub fn palette_defaults() -> &'static [(&'static str, &'static str)] {{"
    )
    .unwrap();
    writeln!(writer, "        &[").unwrap();

    for color in TAILWIND_COLORS {
        let (r, g, b) = color.oklch.to_srgb();
        let name = color.css_var.strip_prefix("--").unwrap_or(color.css_var);
        writeln!(
            writer,
            "            (\"{}\", \"#{:02x}{:02x}{:02x}\"),",
            name, r, g, b
        )
        .unwrap();
    }

    writeln!(writer, "        ]").unwrap();
    writeln!(writer, "    }}").unwrap();
    writeln!(writer, "}}").unwrap();

    // Tell Cargo to rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
}
