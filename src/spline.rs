// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

//! Our objective is to construct a smooth interpolating single-valued function
//! y = f(x).
//!
//! Catmull-Rom splines are widely used for interpolation, however these are
//! parametric curves [x(t) y(t) ...] and can not be used to directly calculate
//! y = f(x).
//! For a discussion of Catmull-Rom splines see Catmull, E., and R. Rom,
//! "A Class of Local Interpolating Splines", Computer Aided Geometric Design.
//!
//! Natural cubic splines are single-valued functions, and have been used in
//! several applications e.g. to specify gamma curves for image display.
//! These splines do not afford local control, and a set of linear equations
//! including all interpolation points must be solved before any point on the
//! curve can be calculated. The lack of local control makes the splines
//! more difficult to handle than e.g. Catmull-Rom splines, and real-time
//! interpolation of a stream of data points is not possible.
//! For a discussion of natural cubic splines, see e.g. Kreyszig, E., "Advanced
//! Engineering Mathematics".
//!
//! Our approach is to approximate the properties of Catmull-Rom splines for
//! piecewice cubic polynomials f(x) = ax^3 + bx^2 + cx + d as follows:
//! Each curve segment is specified by four interpolation points,
//! p0, p1, p2, p3.
//! The curve between p1 and p2 must interpolate both p1 and p2, and in addition
//! f'(p1.x) = k1 = (p2.y - p0.y)/(p2.x - p0.x) and
//! f'(p2.x) = k2 = (p3.y - p1.y)/(p3.x - p1.x).
//!
//! The constraints are expressed by the following system of linear equations
//! ``` ignore,
//! [ 1  xi    xi^2    xi^3 ]   [ d ]    [ yi ]
//! [     1  2*xi    3*xi^2 ] * [ c ] =  [ ki ]
//! [ 1  xj    xj^2    xj^3 ]   [ b ]    [ yj ]
//! [     1  2*xj    3*xj^2 ]   [ a ]    [ kj ]
//! ```
//! Solving using Gaussian elimination and back substitution, setting
//! dy = yj - yi, dx = xj - xi, we get
//! ``` ignore,
//! a = ((ki + kj) - 2*dy/dx)/(dx*dx);
//! b = ((kj - ki)/dx - 3*(xi + xj)*a)/2;
//! c = ki - (3*xi*a + 2*b)*xi;
//! d = yi - ((xi*a + b)*xi + c)*xi;
//! ```
//! Having calculated the coefficients of the cubic polynomial we have the
//! choice of evaluation by brute force
//! ``` ignore,
//! for (x = x1; x <= x2; x += res) {
//!   y = ((a*x + b)*x + c)*x + d;
//!   plot(x, y);
//! }
//! ```
//! or by forward differencing
//! ``` ignore,
//! y = ((a*x1 + b)*x1 + c)*x1 + d;
//! dy = (3*a*(x1 + res) + 2*b)*x1*res + ((a*res + b)*res + c)*res;
//! d2y = (6*a*(x1 + res) + 2*b)*res*res;
//! d3y = 6*a*res*res*res;
//!
//! for (x = x1; x <= x2; x += res) {
//!   plot(x, y);
//!   y += dy; dy += d2y; d2y += d3y;
//! }
//! ```
//! See Foley, Van Dam, Feiner, Hughes, "Computer Graphics, Principles and
//! Practice" for a discussion of forward differencing.
//!
//! If we have a set of interpolation points p0, ..., pn, we may specify
//! curve segments between p0 and p1, and between pn-1 and pn by using the
//! following constraints:
//! f''(p0.x) = 0 and
//! f''(pn.x) = 0.
//!
//! Substituting the results for a and b in
//!
//! 2*b + 6*a*xi = 0
//!
//! we get
//!
//! ki = (3*dy/dx - kj)/2;
//!
//! or by substituting the results for a and b in
//!
//! 2*b + 6*a*xj = 0
//!
//! we get
//!
//! kj = (3*dy/dx - ki)/2;
//!
//! Finally, if we have only two interpolation points, the cubic polynomial
//! will degenerate to a straight line if we set
//!
//! ki = kj = dy/dx;
//!

#![cfg_attr(feature = "cargo-clippy", allow(clippy::float_cmp))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

#[derive(Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Point {
        Point {
            x: x as f64,
            y: y as f64,
        }
    }
}

pub struct PointPlotter<'a> {
    output: &'a mut [i32],
}

impl<'a> PointPlotter<'a> {
    pub fn new(output: &'a mut [i32]) -> Self {
        PointPlotter { output }
    }

    pub fn plot(&mut self, x: f64, y: f64) {
        let value = if y > 0.0 { y as i32 } else { 0 };
        self.output[x as usize] = value;
    }
}

/// Calculation of coefficients.
fn cubic_coefficients(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    k1: f64,
    k2: f64,
) -> (f64, f64, f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let a = ((k1 + k2) - 2.0 * dy / dx) / (dx * dx);
    let b = ((k2 - k1) / dx - 3.0 * (x1 + x2) * a) / 2.0;
    let c = k1 - (3.0 * x1 * a + 2.0 * b) * x1;
    let d = y1 - ((x1 * a + b) * x1 + c) * x1;
    (a, b, c, d)
}

/// Evaluation of cubic polynomial by brute force.
#[allow(dead_code)]
fn interpolate_brute_force(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    k1: f64,
    k2: f64,
    plotter: &mut PointPlotter,
    res: f64,
) {
    let (a, b, c, d) = cubic_coefficients(x1, y1, x2, y2, k1, k2);
    // Calculate each point.
    let mut xi = x1;
    while xi <= x2 {
        let yi = ((a * xi + b) * xi + c) * xi + d;
        plotter.plot(xi, yi);
        xi += res;
    }
}

/// Evaluation of cubic polynomial by forward differencing.
fn interpolate_forward_difference(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    k1: f64,
    k2: f64,
    plotter: &mut PointPlotter,
    res: f64,
) {
    let (a, b, c, d) = cubic_coefficients(x1, y1, x2, y2, k1, k2);
    let mut yi = ((a * x1 + b) * x1 + c) * x1 + d;
    let mut dy = (3.0 * a * (x1 + res) + 2.0 * b) * x1 * res + ((a * res + b) * res + c) * res;
    let mut d2y = (6.0 * a * (x1 + res) + 2.0 * b) * res * res;
    let d3y = 6.0 * a * res * res * res;
    // Calculate each point.
    let mut xi = x1;
    while xi <= x2 {
        plotter.plot(xi, yi);
        yi += dy;
        dy += d2y;
        d2y += d3y;
        xi += res;
    }
}

/// Evaluation of complete interpolating function.
/// Note that since each curve segment is controlled by four points, the
/// end points will not be interpolated. If extra control points are not
/// desirable, the end points can simply be repeated to ensure interpolation.
/// Note also that points of non-differentiability and discontinuity can be
/// introduced by repeating points.
pub fn interpolate<P: Into<Point> + Copy>(points: &[P], plotter: &mut PointPlotter, res: f64) {
    let last_index = points.len() - 4;
    let mut i = 0;
    while i <= last_index {
        let p0 = points[i].into();
        let p1 = points[i + 1].into();
        let p2 = points[i + 2].into();
        let p3 = points[i + 3].into();
        // p1 and p2 equal; single point.
        if p1.x != p2.x {
            let k1;
            let k2;
            if p0.x == p1.x && p2.x == p3.x {
                // Both end points repeated; straight line.
                k1 = (p2.y - p1.y) / (p2.x - p1.x);
                k2 = k1;
            } else if p0.x == p1.x {
                // p0 and p1 equal; use f''(x1) = 0.
                k2 = (p3.y - p1.y) / (p3.x - p1.x);
                k1 = (3.0 * (p2.y - p1.y) / (p2.x - p1.x) - k2) / 2.0;
            } else if p2.x == p3.x {
                // p2 and p3 equal; use f''(x2) = 0.
                k1 = (p2.y - p0.y) / (p2.x - p0.x);
                k2 = (3.0 * (p2.y - p1.y) / (p2.x - p1.x) - k1) / 2.0;
            } else {
                // Normal curve.
                k1 = (p2.y - p0.y) / (p2.x - p0.x);
                k2 = (p3.y - p1.y) / (p3.x - p1.x);
            }
            interpolate_forward_difference(p1.x, p1.y, p2.x, p2.y, k1, k2, plotter, res);
        }
        i += 1;
    }
}
