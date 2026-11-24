//! An avatar from body measurements (PF-10.1): the body the 3D fit preview drapes a pattern on.
//!
//! The body is lofted through horizontal cross-sections (rings) at the heights of the body's
//! landmarks: the base of the neck, the shoulders, the bust, the waist, the hips and the crotch on
//! the torso; the thigh, knee, calf and ankle on each leg; and the top of the arm, the upper arm,
//! the elbow and the wrist on each arm, the arms hanging a little out from the body (an A pose, as
//! fitting avatars stand, so a sleeve and the side of a bodice can hang between arm and body). Each ring is an ellipse as round as the body is
//! there (its depth over its breadth) and exactly as long round as the girth measured, so a
//! garment drafted from the same measurements meets the body where the tape went.
//!
//! Proportions the measurements don't give come from the U.S. Army's ANSUR II survey of 1,986
//! women (2012, public data:
//! <https://ph.health.mil/topics/workplacehealth/ergo/Pages/Anthropometric-Database.aspx>), its
//! means: the chest 0.918 as deep as it is broad, the waist 0.709, the hips (buttock depth over
//! hip breadth) 0.658; the heights of the bust (chest height), waist (omphalion), knee and ankle
//! above the floor, the cervicale's above the waist and the crotch's below the buttocks; the thigh 0.603 of the hips round,
//! the knee (lower thigh) 0.392, the calf 0.366, the ankle 0.212, the base of the neck 0.394 of the
//! chest; the shoulders (acromion) 60.6 mm and the armpit (axilla) 156.7 below the cervicale, the
//! shoulders 365.3 broad (biacromial breadth), 0.386 of the chest round, and as deep at the shoulders
//! as their 1028.2 round over the deltoids at 450.3 across makes them (0.476 of their breadth); the
//! arm 543.6 from the shoulder to the wrist (sleeve outseam), the elbow 311.2 down it
//! (acromion–radiale); the upper arm (biceps) 0.323 of the chest round, the forearm 0.864 of the
//! upper arm and the wrist 0.506 (the means of the survey's women, computed from its public data).
//! A male figure ([`Figure::Male`], PF-10.11) takes the same proportions from the survey's 4,082
//! men instead (in [`MEN`]: a chest 0.877 as deep as broad, hips 0.711, broader shoulders, longer
//! arms, all heights taller), and has no bust standing forward: its chest (the `bust`
//! measurement, taken round the chest) is one elliptic ring. A child ([`Figure::Child`], PF-10.13)
//! takes them from the U.S. Consumer Product Safety Commission's 1977 survey of 3,900 children from
//! 2 to 18 (public domain), by age band, the band chosen by the chest's girth ([`CHILD_BANDS`]).
//! All lengths are in mm, x across the body (to its left), y to its front, z up from the
//! floor.

use std::f64::consts::PI;

/// Points round each ring.
pub const RING_POINTS: usize = 32;

/// One horizontal cross-section of the body: an ellipse at height `z`, centred at `x` across.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    /// What it is: "neck", "bust", "waist", "hip", "crotch", "thigh", "knee", "calf", "ankle", "foot".
    pub name: &'static str,
    /// Height above the floor.
    pub z: f64,
    /// Its middle across the body (0 on the torso; a leg's to one side).
    pub x: f64,
    /// Its middle toward the front (0 but at the bust, which stands forward of a flat back).
    pub y: f64,
    /// Round it.
    pub girth: f64,
    /// Its depth (front to back) over its breadth (side to side).
    pub depth_ratio: f64,
}

impl Ring {
    /// Half its breadth and half its depth: the ellipse `girth` round (Ramanujan's second
    /// approximation of an ellipse's perimeter, within a few parts in a million for a body's).
    pub fn semi_axes(&self) -> (f64, f64) {
        let r = self.depth_ratio;
        let h = ((1.0 - r) / (1.0 + r)).powi(2);
        let per_a = PI * (1.0 + r) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()));
        let a = self.girth / per_a;
        (a, a * r)
    }

    /// Its points, [`RING_POINTS`] of them, starting at the body's left side and going round
    /// through the front.
    pub fn points(&self) -> Vec<[f64; 3]> {
        let (a, b) = self.semi_axes();
        (0..RING_POINTS)
            .map(|i| {
                let t = i as f64 / RING_POINTS as f64 * 2.0 * PI;
                [self.x + a * t.cos(), self.y + b * t.sin(), self.z]
            })
            .collect()
    }
}

/// The measurements an avatar is made from, in mm. Those that are `None` come from the others by
/// ANSUR II's proportions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    pub bust: f64,
    pub waist: f64,
    pub hip: f64,
    pub nape_to_waist: f64,
    pub waist_to_hip: f64,
    pub neck: Option<f64>,
    pub crotch_depth: Option<f64>,
    pub waist_to_knee: Option<f64>,
    pub waist_to_ankle: Option<f64>,
    /// Along the shoulder from the base of the neck to the shoulder point (a bodice's shoulder).
    pub shoulder: Option<f64>,
    /// Round the upper arm.
    pub top_arm: Option<f64>,
    /// From the shoulder point down the outside of the arm to the wrist.
    pub sleeve_length: Option<f64>,
    /// From the nape down to the level of the armpits.
    pub armscye_depth: Option<f64>,
    /// Round the ribcage just under the bust (a bra's band).
    pub underbust: Option<f64>,
    /// Whose proportions fill in the rest.
    pub figure: Figure,
}

/// The measurements an avatar needs: (name, what it is).
pub const MEASUREMENTS: &[(&str, &str)] = &[
    ("bust", "round the fullest part of the bust"),
    ("waist", "round the natural waist"),
    ("hip", "round the fullest part of the hips"),
    ("nape_to_waist", "down the spine from the nape to the waist"),
    ("waist_to_hip", "down the side from the waist to the fullest part of the hips"),
];

/// Why an avatar can't be made.
#[derive(Clone, Debug, PartialEq)]
pub enum AvatarError {
    /// A measurement that is missing, not more than 0, or out of a body's range.
    Measurement(&'static str),
    /// The heights don't go down the body in order (the hips below the crotch, the knee below the
    /// ankle…).
    Order(&'static str),
}

impl std::fmt::Display for AvatarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvatarError::Measurement(m) => write!(f, "the avatar needs your {} measurement", m.replace('_', " ")),
            AvatarError::Order(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for AvatarError {}

/// A triangle mesh: its points and its triangles (each three points' indices, wound so the
/// outside is anticlockwise seen from outside).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// An avatar: its rings (the torso's from the neck down, each leg's from the thigh down and each
/// arm's from the shoulder down) and the mesh lofted through them.
#[derive(Clone, Debug, PartialEq)]
pub struct Avatar {
    pub torso: Vec<Ring>,
    pub legs: [Vec<Ring>; 2],
    /// The body's left arm (to +x) and its right.
    pub arms: [Vec<Ring>; 2],
    pub mesh: Mesh,
    /// Its height to the base of the neck.
    pub neck_height: f64,
}

/// Whose proportions the avatar takes where the measurements don't say: ANSUR II's women's or
/// men's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Figure {
    #[default]
    Female,
    Male,
    /// A child from 2 to 18, the age taken from the chest's girth (PF-10.13).
    Child,
}

impl Figure {
    /// Its name in `measure.set` ("female", "male").
    pub fn name(self) -> &'static str {
        match self {
            Figure::Female => "female",
            Figure::Male => "male",
            Figure::Child => "child",
        }
    }

    pub fn parse(s: &str) -> Option<Figure> {
        match s {
            "female" => Some(Figure::Female),
            "male" => Some(Figure::Male),
            "child" => Some(Figure::Child),
            _ => None,
        }
    }
}

/// A figure's ANSUR II means (mm) and proportions: those in the module docs for the women; the
/// men's (4,082 of them) computed the same way from the same public data; a child's from
/// [`CHILD_BANDS`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct Survey {
    waist_height: f64,
    chest_height: f64,
    cervicale_height: f64,
    crotch_height: f64,
    buttock_height: f64,
    knee_height: f64,
    ankle_height: f64,
    acromial_height: f64,
    axilla_height: f64,
    sleeve_outseam: f64,
    shoulder_to_elbow: f64,
    /// Depth over breadth: the chest, the waist, the hips (buttock depth over hip breadth), the
    /// shoulders (from their girth over the deltoids, over the half breadth at the shoulder points).
    chest_depth: f64,
    waist_depth: f64,
    hip_depth: f64,
    shoulder_depth: f64,
    /// Of the hips round: the thigh, the knee (lower thigh), the calf, the ankle.
    thigh: f64,
    knee: f64,
    calf: f64,
    ankle: f64,
    /// Of the chest round: the base of the neck, the shoulders' breadth, the upper arm (biceps).
    neck: f64,
    shoulders: f64,
    upper_arm: f64,
    /// Of the upper arm round: the forearm, the wrist.
    forearm: f64,
    wrist: f64,
    /// The foot: its length over the ankle bone's height, its breadth over its length.
    foot: f64,
    foot_breadth: f64,
    /// The head (PF-10.15): round over the chest round, long (front to back) over broad, and its top
    /// above the cervicale over the cervicale's height above the waist.
    head: f64,
    head_shape: f64,
    head_height: f64,
    /// The hand: long over the arm (the sleeve outseam), broad over long.
    hand: f64,
    hand_breadth: f64,
    /// The crotch length (from the waist at the navel in front, down between the legs and up to
    /// it behind) over the hips round, and the share of it behind the crotch (PF-10.19).
    crotch_length: f64,
    crotch_back: f64,
    /// The bust stands forward of the ribcage (the women's).
    bust: bool,
}

const WOMEN: Survey = Survey {
    waist_height: 980.0,
    chest_height: 1172.0,
    cervicale_height: 1396.0,
    crotch_height: 782.0,
    buttock_height: 834.0,
    knee_height: 449.0,
    ankle_height: 63.0,
    acromial_height: 1335.1,
    axilla_height: 1239.0,
    sleeve_outseam: 543.6,
    shoulder_to_elbow: 311.2,
    chest_depth: 0.918,
    waist_depth: 0.709,
    hip_depth: 0.658,
    shoulder_depth: 0.476,
    thigh: 0.603,
    knee: 0.392,
    calf: 0.366,
    ankle: 0.212,
    neck: 0.394,
    shoulders: 0.386,
    upper_arm: 0.323,
    forearm: 0.864,
    wrist: 0.506,
    foot: 3.928,
    foot_breadth: 0.376,
    head: 0.593,
    head_shape: 1.285,
    head_height: 0.560,
    hand: 0.333,
    hand_breadth: 0.432,
    crotch_length: 0.6093,
    crotch_back: 0.5509,
    bust: true,
};

const MEN: Survey = Survey {
    waist_height: 1056.5,
    chest_height: 1291.1,
    cervicale_height: 1517.3,
    crotch_height: 845.9,
    buttock_height: 887.9,
    knee_height: 488.4,
    ankle_height: 72.9,
    acromial_height: 1440.7,
    axilla_height: 1329.1,
    sleeve_outseam: 593.1,
    shoulder_to_elbow: 335.2,
    chest_depth: 0.877,
    waist_depth: 0.728,
    hip_depth: 0.711,
    shoulder_depth: 0.503,
    thigh: 0.613,
    knee: 0.401,
    calf: 0.385,
    ankle: 0.225,
    neck: 0.410,
    shoulders: 0.393,
    upper_arm: 0.338,
    forearm: 0.866,
    wrist: 0.491,
    foot: 3.720,
    foot_breadth: 0.376,
    head: 0.543,
    head_shape: 1.293,
    head_height: 0.518,
    hand: 0.326,
    hand_breadth: 0.457,
    crotch_length: 0.6114,
    crotch_back: 0.5728,
    bust: false,
};

/// The children's survey: the U.S. Consumer Product Safety Commission's measurements of 3,900
/// children from 2 to 18 (Snyder et al., "Anthropometry of Infants, Children, and Youths to Age 18
/// for Product Safety Design", University of Michigan, 1977), public domain, its records at NIST:
/// <https://math.nist.gov/~SRessler/anthrokids/> (`orig77/individuals.csv`). Each band's means,
/// computed from the records (a 0 is a measurement not taken, left out); the heights the survey
/// doesn't have from those it does by ANSUR II's women's proportions between the same landmarks:
/// the cervicale above the waist 1.189 times the suprasternale's (415.6 over 349.6), the chest
/// (bust) below the armpit 0.260 of the armpit's height above the waist, the shoulder point above
/// the armpit 0.613 of the way to the cervicale, the ankle bone 0.0385 of the stature; the crotch
/// at the gluteal furrow, the knee at the tibiale; the arm's outseam the shoulder to the elbow and
/// the elbow to the hand less the hand. Depths over breadths from each girth and the breadth
/// measured with it (an ellipse that long round and that broad): the chest's at the armpit (as
/// round as a circle at most), the waist's, the hips' from the seated hip breadth. The knee (lower
/// thigh) 0.650 of the thigh, the shoulders' breadth at the shoulder points 0.811 of the breadth
/// over the deltoids, and the shoulders' depth: ANSUR II's women's. The bands are chosen by the
/// chest's girth, between them in proportion (no age to enter); none has a bust.
/// Each band: its mean chest round and its proportions.
const CHILD_BANDS: [(f64, Survey); 8] = [
    // Ages 2 to 4 (330 children).
    (
        511.6,
        Survey {
            waist_height: 510.9,
            chest_height: 629.6,
            cervicale_height: 782.1,
            crotch_height: 384.6,
            buttock_height: 420.3,
            knee_height: 226.2,
            ankle_height: 36.9,
            acromial_height: 739.3,
            axilla_height: 671.4,
            sleeve_outseam: 332.5,
            shoulder_to_elbow: 189.7,
            chest_depth: 1.000,
            waist_depth: 0.857,
            hip_depth: 0.704,
            shoulder_depth: 0.476,
            thigh: 0.567,
            knee: 0.368,
            calf: 0.397,
            ankle: 0.283,
            neck: 0.466,
            shoulders: 0.393,
            upper_arm: 0.311,
            forearm: 0.991,
            wrist: 0.712,
            foot: 4.106,
            foot_breadth: 0.410,
            head: 0.971,
            head_shape: 1.308,
            head_height: 0.646,
            hand: 0.324,
            hand_breadth: 0.481,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 4 to 6 (514 children).
    (
        551.1,
        Survey {
            waist_height: 607.4,
            chest_height: 737.9,
            cervicale_height: 890.2,
            crotch_height: 462.6,
            buttock_height: 508.0,
            knee_height: 265.3,
            ankle_height: 41.8,
            acromial_height: 849.1,
            axilla_height: 783.8,
            sleeve_outseam: 383.3,
            shoulder_to_elbow: 218.6,
            chest_depth: 1.000,
            waist_depth: 0.827,
            hip_depth: 0.710,
            shoulder_depth: 0.476,
            thigh: 0.581,
            knee: 0.377,
            calf: 0.395,
            ankle: 0.277,
            neck: 0.455,
            shoulders: 0.396,
            upper_arm: 0.304,
            forearm: 0.991,
            wrist: 0.709,
            foot: 4.080,
            foot_breadth: 0.400,
            head: 0.921,
            head_shape: 1.304,
            head_height: 0.688,
            hand: 0.315,
            hand_breadth: 0.468,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 6 to 8 (447 children).
    (
        597.4,
        Survey {
            waist_height: 695.2,
            chest_height: 839.2,
            cervicale_height: 1003.7,
            crotch_height: 541.2,
            buttock_height: 586.6,
            knee_height: 306.8,
            ankle_height: 46.5,
            acromial_height: 959.7,
            axilla_height: 889.9,
            sleeve_outseam: 432.3,
            shoulder_to_elbow: 246.5,
            chest_depth: 1.000,
            waist_depth: 0.799,
            hip_depth: 0.722,
            shoulder_depth: 0.476,
            thigh: 0.587,
            knee: 0.382,
            calf: 0.394,
            ankle: 0.270,
            neck: 0.438,
            shoulders: 0.399,
            upper_arm: 0.300,
            forearm: 0.984,
            wrist: 0.694,
            foot: 4.059,
            foot_breadth: 0.393,
            head: 0.862,
            head_shape: 1.300,
            head_height: 0.665,
            hand: 0.306,
            hand_breadth: 0.464,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 8 to 10 (491 children).
    (
        656.3,
        Survey {
            waist_height: 789.3,
            chest_height: 945.5,
            cervicale_height: 1116.5,
            crotch_height: 615.9,
            buttock_height: 669.6,
            knee_height: 347.1,
            ankle_height: 51.2,
            acromial_height: 1071.6,
            axilla_height: 1000.5,
            sleeve_outseam: 482.5,
            shoulder_to_elbow: 274.7,
            chest_depth: 1.000,
            waist_depth: 0.791,
            hip_depth: 0.717,
            shoulder_depth: 0.476,
            thigh: 0.595,
            knee: 0.387,
            calf: 0.392,
            ankle: 0.264,
            neck: 0.422,
            shoulders: 0.397,
            upper_arm: 0.301,
            forearm: 0.968,
            wrist: 0.667,
            foot: 4.068,
            foot_breadth: 0.386,
            head: 0.798,
            head_shape: 1.304,
            head_height: 0.654,
            hand: 0.300,
            hand_breadth: 0.458,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 10 to 12 (541 children).
    (
        705.2,
        Survey {
            waist_height: 856.8,
            chest_height: 1021.1,
            cervicale_height: 1218.7,
            crotch_height: 677.4,
            buttock_height: 727.3,
            knee_height: 381.0,
            ankle_height: 55.2,
            acromial_height: 1164.6,
            axilla_height: 1079.0,
            sleeve_outseam: 524.8,
            shoulder_to_elbow: 298.2,
            chest_depth: 1.000,
            waist_depth: 0.759,
            hip_depth: 0.726,
            shoulder_depth: 0.476,
            thigh: 0.597,
            knee: 0.388,
            calf: 0.389,
            ankle: 0.253,
            neck: 0.404,
            shoulders: 0.396,
            upper_arm: 0.301,
            forearm: 0.961,
            wrist: 0.646,
            foot: 4.065,
            foot_breadth: 0.384,
            head: 0.754,
            head_shape: 1.288,
            head_height: 0.597,
            hand: 0.298,
            hand_breadth: 0.454,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 12 to 14 (599 children).
    (
        759.2,
        Survey {
            waist_height: 939.0,
            chest_height: 1118.9,
            cervicale_height: 1309.0,
            crotch_height: 732.8,
            buttock_height: 796.6,
            knee_height: 410.8,
            ankle_height: 59.5,
            acromial_height: 1260.0,
            axilla_height: 1182.3,
            sleeve_outseam: 570.6,
            shoulder_to_elbow: 323.3,
            chest_depth: 1.000,
            waist_depth: 0.727,
            hip_depth: 0.713,
            shoulder_depth: 0.476,
            thigh: 0.593,
            knee: 0.385,
            calf: 0.387,
            ankle: 0.254,
            neck: 0.397,
            shoulders: 0.394,
            upper_arm: 0.302,
            forearm: 0.956,
            wrist: 0.641,
            foot: 4.023,
            foot_breadth: 0.386,
            head: 0.711,
            head_shape: 1.284,
            head_height: 0.638,
            hand: 0.296,
            hand_breadth: 0.451,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 14 to 16 (490 children).
    (
        824.1,
        Survey {
            waist_height: 996.0,
            chest_height: 1183.9,
            cervicale_height: 1387.5,
            crotch_height: 767.4,
            buttock_height: 840.0,
            knee_height: 428.7,
            ankle_height: 63.1,
            acromial_height: 1334.3,
            axilla_height: 1250.1,
            sleeve_outseam: 607.6,
            shoulder_to_elbow: 343.7,
            chest_depth: 1.000,
            waist_depth: 0.702,
            hip_depth: 0.703,
            shoulder_depth: 0.476,
            thigh: 0.592,
            knee: 0.385,
            calf: 0.382,
            ankle: 0.243,
            neck: 0.388,
            shoulders: 0.393,
            upper_arm: 0.304,
            forearm: 0.945,
            wrist: 0.619,
            foot: 3.929,
            foot_breadth: 0.387,
            head: 0.666,
            head_shape: 1.287,
            head_height: 0.645,
            hand: 0.292,
            hand_breadth: 0.452,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
    // Ages 16 to 19 (483 children).
    (
        878.7,
        Survey {
            waist_height: 1026.2,
            chest_height: 1223.2,
            cervicale_height: 1438.0,
            crotch_height: 782.7,
            buttock_height: 861.6,
            knee_height: 441.7,
            ankle_height: 65.3,
            acromial_height: 1381.7,
            axilla_height: 1292.5,
            sleeve_outseam: 627.4,
            shoulder_to_elbow: 354.5,
            chest_depth: 1.000,
            waist_depth: 0.712,
            hip_depth: 0.700,
            shoulder_depth: 0.476,
            thigh: 0.593,
            knee: 0.386,
            calf: 0.381,
            ankle: 0.237,
            neck: 0.384,
            shoulders: 0.391,
            upper_arm: 0.306,
            forearm: 0.935,
            wrist: 0.597,
            foot: 3.868,
            foot_breadth: 0.388,
            head: 0.634,
            head_shape: 1.277,
            head_height: 0.626,
            hand: 0.289,
            hand_breadth: 0.455,
            crotch_length: WOMEN.crotch_length,
            crotch_back: WOMEN.crotch_back,
            bust: false,
        },
    ),
];

/// A child's survey for a chest `chest` round: between the two bands it falls between, in
/// proportion, else the youngest's or the oldest's.
fn child_survey(chest: f64) -> Survey {
    let k = CHILD_BANDS.iter().position(|(c, _)| *c > chest).unwrap_or(CHILD_BANDS.len());
    if k == 0 {
        return CHILD_BANDS[0].1;
    }
    if k == CHILD_BANDS.len() {
        return CHILD_BANDS[k - 1].1;
    }
    let ((c0, a), (c1, b)) = (CHILD_BANDS[k - 1], CHILD_BANDS[k]);
    let t = (chest - c0) / (c1 - c0);
    let mix = |x: f64, y: f64| x + (y - x) * t;
    Survey {
        waist_height: mix(a.waist_height, b.waist_height),
        chest_height: mix(a.chest_height, b.chest_height),
        cervicale_height: mix(a.cervicale_height, b.cervicale_height),
        crotch_height: mix(a.crotch_height, b.crotch_height),
        buttock_height: mix(a.buttock_height, b.buttock_height),
        knee_height: mix(a.knee_height, b.knee_height),
        ankle_height: mix(a.ankle_height, b.ankle_height),
        acromial_height: mix(a.acromial_height, b.acromial_height),
        axilla_height: mix(a.axilla_height, b.axilla_height),
        sleeve_outseam: mix(a.sleeve_outseam, b.sleeve_outseam),
        shoulder_to_elbow: mix(a.shoulder_to_elbow, b.shoulder_to_elbow),
        chest_depth: mix(a.chest_depth, b.chest_depth),
        waist_depth: mix(a.waist_depth, b.waist_depth),
        hip_depth: mix(a.hip_depth, b.hip_depth),
        shoulder_depth: mix(a.shoulder_depth, b.shoulder_depth),
        thigh: mix(a.thigh, b.thigh),
        knee: mix(a.knee, b.knee),
        calf: mix(a.calf, b.calf),
        ankle: mix(a.ankle, b.ankle),
        neck: mix(a.neck, b.neck),
        shoulders: mix(a.shoulders, b.shoulders),
        upper_arm: mix(a.upper_arm, b.upper_arm),
        forearm: mix(a.forearm, b.forearm),
        wrist: mix(a.wrist, b.wrist),
        foot: mix(a.foot, b.foot),
        foot_breadth: mix(a.foot_breadth, b.foot_breadth),
        head: mix(a.head, b.head),
        head_shape: mix(a.head_shape, b.head_shape),
        head_height: mix(a.head_height, b.head_height),
        hand: mix(a.hand, b.hand),
        hand_breadth: mix(a.hand_breadth, b.hand_breadth),
        crotch_length: mix(a.crotch_length, b.crotch_length),
        crotch_back: mix(a.crotch_back, b.crotch_back),
        bust: false,
    }
}

/// The height from the waist to the cervicale over the length down the back between them
/// (cervicale height less waist height, 415.5, over the waist back length, 425.4; the men's
/// 460.8 over 477.6, the same to within 0.3%).
pub const BACK_RISE: f64 = 415.5 / 425.4;
/// How far out from straight down the arms hang (an A pose).
/// A B cup's bust over underbust (EN 13402-3: 14 to 16 cm), when the set has no underbust.
const B_CUP: f64 = 150.0;
/// How far below the bust point the underbust is (a bra's band; chosen: the survey doesn't
/// measure it), and how deep the ribcage is there over its breadth (chosen, between the chest's and
/// the waist's).
const UNDERBUST_DROP: f64 = 75.0;
const UNDERBUST_DEPTH: f64 = 0.8;
const ARM_ANGLE: f64 = 30.0 * PI / 180.0;

impl Avatar {
    /// The torso's ring called `name` ("neck", "shoulder", "bust", "waist", "hip", "crotch").
    pub fn ring(&self, name: &str) -> Option<&Ring> {
        self.torso.iter().find(|r| r.name == name)
    }

    /// Each chain of rings the body is lofted through: the torso, the legs, the arms.
    pub fn chains(&self) -> [&Vec<Ring>; 5] {
        [&self.torso, &self.legs[0], &self.legs[1], &self.arms[0], &self.arms[1]]
    }
}

/// The girth of an ellipse with semi-axes `a` and `b` (Ramanujan's second approximation).
/// The body's crotch length in front and behind (PF-10.19): the survey's share of the hips round
/// (ANSUR II: the crotch length from the navel, 620.8 mm for the women's mean buttock girth of
/// 1021.2, 0.609; the men's 0.611), longer on each side by how much further the body's waist is
/// above its crotch (`rise`) than the survey's navel above its crotch, for the body's size (a
/// dressmaker's waist is above the navel), split in the survey's share behind (0.551, 0.573).
fn crotch_lengths(hip: f64, rise: f64, sv: &Survey, s: f64) -> (f64, f64) {
    let above = (rise - (sv.waist_height - sv.crotch_height) * s).max(0.0);
    let length = sv.crotch_length * hip;
    (length * (1.0 - sv.crotch_back) + above, length * sv.crotch_back + above)
}

/// How far it is down the middle of the front (`side` 1) or back (-1) from the waist, over the
/// hips, to the crotch ring's front or back and on under it to the crotch point between them: the
/// rings lofted straight, as the body is (see [`centre_lengths`]).
fn down_the_middle(rings: &[Ring], side: f64) -> f64 {
    let edge = |r: &Ring| (r.y + side * r.semi_axes().1, r.z);
    let mut length = 0.0;
    for w in rings.windows(2) {
        let ((y0, z0), (y1, z1)) = (edge(&w[0]), edge(&w[1]));
        length += (y1 - y0).hypot(z1 - z0);
    }
    length + rings.last().map_or(0.0, |r| r.semi_axes().1)
}

/// The avatar's crotch length in front and behind, down the middle from the waist through the
/// crotch (PF-10.19): its torso's rings from the waist down.
pub fn centre_lengths(a: &Avatar) -> Option<(f64, f64)> {
    let from = a.torso.iter().position(|r| r.name == "waist")?;
    let rings = &a.torso[from..];
    (rings.last()?.name == "crotch").then(|| (down_the_middle(rings, 1.0), down_the_middle(rings, -1.0)))
}

/// The crotch ring (PF-10.19): below the hips the body curves in under the belly in front and
/// under the buttocks behind to the crotch between the legs; its front and back drawn in from the
/// hips' (as broad across as `full`, the hips carried on down) until the way down the middle from
/// the waist through the crotch is `front + back` long, shared as near `(front, back)` as the
/// shape allows. Never deeper than `full`; no shallower than 40 mm (a body's thighs meet below
/// it).
fn crotch(waist: &Ring, hip: &Ring, full: Ring, (front, back): (f64, f64)) -> Ring {
    const SHALLOWEST: f64 = 40.0;
    let (a, b0) = full.semi_axes();
    let shaped = |f: f64, bk: f64| {
        let b = ((f - bk) / 2.0).max(1.0);
        Ring { y: (f + bk) / 2.0, girth: ellipse_girth(a, b), depth_ratio: b / a, ..full }
    };
    // The whole length first, then how it is shared (the shape can't always give both: a deep
    // crotch drawn in far enough in front leaves the back short).
    let miss = |f: f64, bk: f64| {
        let rings = [*waist, *hip, shaped(f, bk)];
        let (df, db) = (down_the_middle(&rings, 1.0) - front, down_the_middle(&rings, -1.0) - back);
        100.0 * (df + db).powi(2) + (df - db).powi(2)
    };
    // The front from the hips' front back, the back from the hips' back forward: the pair that
    // misses least, coarse then fine.
    let (mut f, mut bk) = (b0, -b0);
    let mut step = b0 / 20.0;
    for _ in 0..6 {
        let (cf, cb) = (f, bk);
        for i in -20..=20 {
            for j in -20..=20 {
                let (tf, tb) = ((cf + i as f64 * step).min(b0), (cb + j as f64 * step).max(-b0));
                if tf - tb >= SHALLOWEST && miss(tf, tb) < miss(f, bk) {
                    (f, bk) = (tf, tb);
                }
            }
        }
        step /= 10.0;
    }
    shaped(f, bk)
}

/// The gap between the thighs at their tops, mm: two layers of cloth's margins and a little.
const THIGH_GAP: f64 = 5.0;

/// The half-depth of the ellipse `a` across (half) that is `girth` round (at least a tenth of `a`).
fn depth_for(a: f64, girth: f64) -> f64 {
    let (mut lo, mut hi) = (a * 0.1, a * 10.0);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if ellipse_girth(a, mid) < girth { lo = mid } else { hi = mid }
    }
    (lo + hi) / 2.0
}

fn ellipse_girth(a: f64, b: f64) -> f64 {
    let h = ((a - b) / (a + b)).powi(2);
    PI * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()))
}

/// Make the avatar for `body`.
pub fn avatar(body: &Body) -> Result<Avatar, AvatarError> {
    let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    for (name, v, lo, hi) in [
        ("bust", body.bust, 400.0, 2500.0),
        ("waist", body.waist, 300.0, 2500.0),
        ("hip", body.hip, 400.0, 2500.0),
        ("nape_to_waist", body.nape_to_waist, 200.0, 800.0),
        ("waist_to_hip", body.waist_to_hip, 50.0, 500.0),
    ] {
        if !ok(v, lo, hi) {
            return Err(AvatarError::Measurement(name));
        }
    }
    // The body's scale along its length: the nape to the waist against the survey's.
    // The nape to the waist is measured down the curve of the back: its height is a little less.
    let child;
    let sv = match body.figure {
        Figure::Female => &WOMEN,
        Figure::Male => &MEN,
        Figure::Child => {
            child = child_survey(body.bust);
            &child
        }
    };
    let rise = body.nape_to_waist * BACK_RISE;
    let s = rise / (sv.cervicale_height - sv.waist_height);
    let waist_z = match body.waist_to_ankle {
        Some(w) => w + sv.ankle_height * s,
        None => sv.waist_height * s,
    };
    // Without a crotch depth, the crotch the survey's way below the fullest part of the hips (its
    // waist is the navel's, lower than a dressmaker's natural waist, so not from the waist).
    let crotch_z = match body.crotch_depth {
        Some(c) => waist_z - c,
        None => waist_z - body.waist_to_hip - (sv.buttock_height - sv.crotch_height) * s,
    };
    let knee_z = match body.waist_to_knee {
        Some(k) => waist_z - k,
        None => sv.knee_height * (crotch_z / sv.crotch_height),
    };
    let ankle_z = sv.ankle_height * s;
    let hip_z = waist_z - body.waist_to_hip;
    let neck_z = waist_z + rise;
    let bust_z = waist_z + (sv.chest_height - sv.waist_height) * s;
    if hip_z <= crotch_z + 10.0 {
        return Err(AvatarError::Order("the hips are at or below the crotch: waist_to_hip is less than crotch_depth"));
    }
    if crotch_z <= knee_z + 50.0 || knee_z <= ankle_z + 50.0 {
        return Err(AvatarError::Order("the crotch, knee and ankle don't go down the leg in order"));
    }
    let ring = |name, z, x, girth, depth_ratio| Ring { name, z, x, y: 0.0, girth, depth_ratio };
    let neck = body.neck.unwrap_or(body.bust * sv.neck);
    // The shoulders: as broad as two shoulder lengths and the neck across, else the survey's share
    // of the chest; as deep as the survey's.
    let shoulder_z = neck_z - (sv.cervicale_height - sv.acromial_height) * s;
    // The armpits as deep below the nape as measured (a bodice's underarm is drafted there),
    // else the survey's.
    let axilla_z = match body.armscye_depth {
        Some(d) if d > 0.0 => neck_z - d * BACK_RISE,
        _ => neck_z - (sv.cervicale_height - sv.axilla_height) * s,
    };
    let breadth = match body.shoulder {
        Some(l) if l > 0.0 => 2.0 * l + neck / PI,
        _ => body.bust * sv.shoulders,
    };
    let shoulder_ring = ring("shoulder", shoulder_z, 0.0, ellipse_girth(breadth / 2.0, breadth / 2.0 * sv.shoulder_depth), sv.shoulder_depth);
    // The bust: the ribcage under it as round as the underbust (else a B cup's, EN 13402-3: the
    // bust 14 to 16 cm more round), the back flat from the underbust up to the bust, the bust's
    // girth more than the ribcage's standing forward.
    // A man's chest (the `bust` measurement, round the chest) has none: one elliptic ring.
    let underbust = body.underbust.filter(|u| *u > 0.0 && *u < body.bust).unwrap_or((body.bust - B_CUP).max(body.bust * 0.75));
    let under = ring("underbust", bust_z - UNDERBUST_DROP * s, 0.0, underbust, UNDERBUST_DEPTH);
    let mut bust_ring = ring("bust", bust_z, 0.0, body.bust, sv.chest_depth);
    if sv.bust {
        bust_ring.y = bust_ring.semi_axes().1 - under.semi_axes().1;
    }
    // The torso: the hips' girth carried on to the crotch, a little in, where the legs part.
    // The head (PF-10.15): above the base of the neck as high as the survey's head is above its
    // cervicale for the body's rise, as round as its share of the chest, as long front to back over
    // broad; lofted through the throat, the jaw, the head at its roundest and the crown, a little
    // forward of the neck. (Chosen: where the rings sit up the head, how round each is.) Drawn and
    // exported; the cloth collides with the body from the base of the neck down.
    let head_round = sv.head * body.bust;
    let head_h = sv.head_height * rise;
    let length = ring("head", 0.0, 0.0, head_round, sv.head_shape).semi_axes().1 * 2.0;
    let head_ring = |name, at: f64, girth: f64, shape: f64, forward: f64| {
        let mut r = ring(name, neck_z + head_h * at, 0.0, girth, shape);
        r.y = length * forward;
        r
    };
    let mut torso = vec![
        head_ring("crown_top", 0.99, head_round * 0.3, sv.head_shape, 0.02),
        head_ring("crown", 0.94, head_round * 0.66, sv.head_shape, 0.03),
        head_ring("temple", 0.82, head_round * 0.93, sv.head_shape, 0.04),
        head_ring("head", 0.64, head_round, sv.head_shape, 0.05),
        head_ring("jaw", 0.38, head_round * 0.86, 1.2, 0.08),
        head_ring("throat", 0.15, neck, 1.0, 0.03),
        ring("neck", neck_z, 0.0, neck, 1.0),
        shoulder_ring,
        bust_ring,
    ];
    if sv.bust {
        torso.push(under);
    }
    let hip_ring = ring("hip", hip_z, 0.0, body.hip, sv.hip_depth);
    let waist_ring = ring("waist", waist_z, 0.0, body.waist, sv.waist_depth);
    let crotch_ring = crotch(
        &waist_ring,
        &hip_ring,
        ring("crotch", crotch_z, 0.0, body.hip * 0.96, sv.hip_depth),
        crotch_lengths(body.hip, waist_z - crotch_z, sv, s),
    );
    torso.extend([waist_ring, hip_ring, crotch_ring]);
    // Each leg: its thigh just below the crotch, side by side across the hips, its outside in line
    // with the side of the hips and its inside just clear of the other's (PF-10.19: the thighs
    // touch at the top, as they do on many bodies; overlapping across the middle they filled the
    // crotch, and a trouser's crotch seam, pushed down out of them, hung 65 mm below the body's
    // crotch and the trousers with it), as round as the survey's thigh, so deeper than broad.
    let (hip_a, _) = hip_ring.semi_axes();
    let thigh_a = (hip_a - THIGH_GAP) / 2.0;
    let thigh_b = depth_for(thigh_a, body.hip * sv.thigh);
    // Each foot (PF-10.14): under the ankle a ring as long as the survey's foot (front to back) and
    // as broad, a quarter of it behind the ankle; low, at a fifth of the ankle bone's height, the
    // leg closed under it. Garments' hems rest on it, and on the floor.
    let foot = |x: f64| {
        let length = ankle_z * sv.foot;
        let (a, b) = (length * sv.foot_breadth / 2.0, length / 2.0);
        let mut r = ring("foot", ankle_z * 0.2, x, ellipse_girth(a, b), b / a);
        r.y = length * 0.25;
        r
    };
    let leg = |side: f64| {
        let x = side * hip_a / 2.0;
        let calf_z = knee_z - (knee_z - ankle_z) * 0.3;
        vec![
            ring("thigh", crotch_z - 10.0, x, ellipse_girth(thigh_a, thigh_b), thigh_b / thigh_a),
            ring("knee", knee_z, x, body.hip * sv.knee, 0.95),
            ring("calf", calf_z, x, body.hip * sv.calf, 0.95),
            ring("ankle", ankle_z, x, body.hip * sv.ankle, 0.9),
            foot(x),
        ]
    };
    let legs = [leg(1.0), leg(-1.0)];
    let upper = body.top_arm.filter(|g| *g > 0.0).unwrap_or(body.bust * sv.upper_arm);
    let reach = body.sleeve_length.filter(|l| *l > 0.0).map_or(s, |l| l / sv.sleeve_outseam);
    // Each arm swings out from its shoulder joint (the head of the upper arm, under the shoulder
    // point), as an arm does: the top of the arm is the deltoid, round over the joint and on out
    // past the shoulder point (the survey's shoulders are 450.3 across the deltoids for 365.3
    // across the shoulder points); below it the arm hangs out at the A pose's angle, the armpit
    // opening under it. Seen across, a tilted arm's level cross-section is longer by 1/cos.
    let deltoid = upper * 1.12;
    let r_top = deltoid / (2.0 * PI);
    let angle = ARM_ANGLE;
    // The joint sits in from the shoulder point by a fifth of the deltoid's radius (further in,
    // or the arm at 20°, and the sleeves drag the bodice down the arms: PF-10.9's debug log).
    let joint = (breadth / 2.0 - r_top * 0.2, shoulder_z - r_top);
    // A point down the arm's line from the joint, `l` along it.
    let along = |l: f64| (joint.0 + l * angle.sin(), joint.1 - l * angle.cos());
    let at_z = |z: f64| joint.0 + (joint.1 - z) * angle.tan();
    let top_z = shoulder_z - r_top * 0.25;
    let elbow = along(sv.shoulder_to_elbow * reach - r_top);
    let wrist = along(sv.sleeve_outseam * reach - r_top);
    let hand_len = sv.hand * sv.sleeve_outseam * reach;
    let palm = along(sv.sleeve_outseam * reach - r_top + hand_len * 0.45);
    let fingers = along(sv.sleeve_outseam * reach - r_top + hand_len * 0.8);
    let tips = along(sv.sleeve_outseam * reach - r_top + hand_len * 0.97);
    // How thick the hand is over how broad (chosen: a flat hand).
    let hand_flat = 0.45 * (0.45 / sv.hand_breadth);
    let tilted = |name, z: f64, x: f64, girth: f64, depth: f64| {
        let r = girth / (2.0 * PI);
        let (a, b) = (r / angle.cos(), r * depth);
        ring(name, z, x, ellipse_girth(a, b), b / a)
    };
    let arm = |side: f64| {
        // Over the deltoid a rounded top, so a sleeve's cap rests on it (the body is only where
        // its rings are: without it the arm's top was open to the cloth, and the cap slid off it).
        let cap_z = top_z + r_top * 0.55;
        vec![
            ring("armcap", cap_z, side * (joint.0 - (cap_z - joint.1) * angle.tan()), deltoid * 0.55, 0.9),
            ring("armtop", top_z, side * (joint.0 - (top_z - joint.1) * angle.tan()), deltoid, 0.9),
            tilted("upperarm", axilla_z, side * at_z(axilla_z), upper, 0.95),
            tilted("elbow", elbow.1, side * elbow.0, upper * sv.forearm, 0.95),
            tilted("wrist", wrist.1, side * wrist.0, upper * sv.wrist, 0.75),
            // The hand (PF-10.15) on along the arm's line: its palm as round as the wrist and a
            // half, flat; the fingers; their tips.
            tilted("palm", palm.1, side * palm.0, upper * sv.wrist * 1.45, hand_flat),
            tilted("fingers", fingers.1, side * fingers.0, upper * sv.wrist * 1.05, hand_flat),
            tilted("fingertips", tips.1, side * tips.0, upper * sv.wrist * 0.45, hand_flat),
        ]
    };
    let (elbow_z, wrist_z) = (elbow.1, wrist.1);
    let arms = [arm(1.0), arm(-1.0)];
    if !(top_z > axilla_z && axilla_z > elbow_z + 50.0 && elbow_z > wrist_z + 50.0) {
        return Err(AvatarError::Order("the shoulder, armpit, elbow and wrist don't go down the arm in order"));
    }
    let mut mesh = Mesh::default();
    for rings in [&torso, &legs[0], &legs[1], &arms[0], &arms[1]] {
        loft(&mut mesh, rings);
    }
    Ok(Avatar { torso, legs, arms, mesh, neck_height: neck_z })
}

/// A tube through `rings` (top to bottom) added to `mesh`, its ends closed with a fan.
fn loft(mesh: &mut Mesh, rings: &[Ring]) {
    let n = RING_POINTS as u32;
    let start = mesh.vertices.len() as u32;
    for r in rings {
        mesh.vertices.extend(r.points());
    }
    for k in 0..rings.len().saturating_sub(1) as u32 {
        let (up, down) = (start + k * n, start + (k + 1) * n);
        for i in 0..n {
            let j = (i + 1) % n;
            // Seen from outside the body, a ring runs anticlockwise (left side, front, right).
            mesh.triangles.push([up + i, down + i, down + j]);
            mesh.triangles.push([up + i, down + j, up + j]);
        }
    }
    // The ends: the top seen from above, the bottom from below.
    for (k, top) in [(0usize, true), (rings.len().saturating_sub(1), false)] {
        let Some(r) = rings.get(k) else { continue };
        let c = mesh.vertices.len() as u32;
        mesh.vertices.push([r.x, 0.0, r.z]);
        let base = start + k as u32 * n;
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.triangles.push(if top { [c, base + i, base + j] } else { [c, base + j, base + i] });
        }
    }
}

/// The mesh as a Wavefront OBJ file, in metres (as glTF and most 3D software take them), y up
/// and z to the front, as OBJ files are usually read.
pub fn to_obj(mesh: &Mesh, name: &str) -> String {
    let mut out = String::with_capacity(mesh.vertices.len() * 40 + mesh.triangles.len() * 24);
    out.push_str("# PatternFoundry avatar, metres, y up\n");
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    out.push_str(&format!("o {}\n", if safe.is_empty() { "avatar".into() } else { safe }));
    for v in &mesh.vertices {
        out.push_str(&format!("v {:.5} {:.5} {:.5}\n", v[0] / 1000.0, v[2] / 1000.0, v[1] / 1000.0));
    }
    for t in &mesh.triangles {
        // OBJ counts from 1; swapping y and z mirrors the mesh, so each triangle is turned back.
        out.push_str(&format!("f {} {} {}\n", t[0] + 1, t[2] + 1, t[1] + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Body {
        Body { bust: 880.0, waist: 680.0, hip: 960.0, nape_to_waist: 410.0, waist_to_hip: 200.0, ..Default::default() }
    }

    /// The perimeter of ring `r`'s polygon of many points: close to the ellipse's.
    fn round_it(r: &Ring) -> f64 {
        let (a, b) = r.semi_axes();
        let n = 20_000;
        (0..n)
            .map(|i| {
                let (t0, t1) = (i as f64 / n as f64 * 2.0 * PI, (i + 1) as f64 / n as f64 * 2.0 * PI);
                ((a * t1.cos() - a * t0.cos()).powi(2) + (b * t1.sin() - b * t0.sin()).powi(2)).sqrt()
            })
            .sum()
    }

    #[test]
    fn each_ring_is_as_long_round_as_its_girth_and_as_deep_as_the_body() {
        let a = avatar(&body()).unwrap();
        for r in a.torso.iter().chain(a.legs.iter().flatten()).chain(a.arms.iter().flatten()) {
            assert!((round_it(r) - r.girth).abs() / r.girth < 1e-5, "{}: {} against {}", r.name, round_it(r), r.girth);
            let (ax, by) = r.semi_axes();
            assert!((by / ax - r.depth_ratio).abs() < 1e-12);
        }
        let by = |n: &str| a.torso.iter().find(|r| r.name == n).copied().unwrap();
        assert_eq!((by("bust").girth, by("waist").girth, by("hip").girth), (880.0, 680.0, 960.0));
        // The heights: the nape 410 down the back above the waist (a little less straight up), the
        // hips 200 below it, in order down the leg.
        assert!((by("neck").z - by("waist").z - 410.0 * BACK_RISE).abs() < 1e-9);
        assert!((by("waist").z - by("hip").z - 200.0).abs() < 1e-9);
        let zs: Vec<f64> = a.torso.iter().chain(&a.legs[0]).map(|r| r.z).collect();
        assert!(zs.windows(2).all(|w| w[0] > w[1]), "{zs:?}");
        // The survey's woman: her cervicale 416 above her waist puts her waist at 980.
        let mean = avatar(&Body { nape_to_waist: 416.0 / BACK_RISE, ..body() }).unwrap();
        assert!((mean.ring("waist").unwrap().z - 980.0).abs() < 1e-9);
    }

    #[test]
    fn given_lengths_are_used_and_bad_measurements_refused() {
        let a = avatar(&Body { waist_to_ankle: Some(900.0), waist_to_knee: Some(560.0), crotch_depth: Some(260.0), ..body() }).unwrap();
        let waist = a.ring("waist").unwrap().z;
        let ankle = a.legs[0][3].z;
        assert!((waist - ankle - 900.0).abs() < 1e-9 && (waist - a.legs[0][1].z - 560.0).abs() < 1e-9);
        assert!((waist - a.ring("crotch").unwrap().z - 260.0).abs() < 1e-9);
        // The legs apart, either side of the middle.
        assert!(a.legs[0][0].x > 0.0 && a.legs[1][0].x < 0.0);
        assert_eq!(avatar(&Body { bust: 0.0, ..body() }), Err(AvatarError::Measurement("bust")));
        assert!(matches!(avatar(&Body { crotch_depth: Some(150.0), ..body() }), Err(AvatarError::Order(_))));
    }

    /// The shoulders as broad as two shoulder lengths and the neck, below the base of the neck by
    /// the survey's 60.6 mm; the arms down to the wrist as long as the sleeve length, hanging out
    /// clear of the chest and the hips, the upper arm as round as measured.
    #[test]
    fn the_shoulders_and_arms_follow_the_measurements() {
        let b = Body { shoulder: Some(121.0), neck: Some(370.0), top_arm: Some(280.0), sleeve_length: Some(590.0), ..body() };
        let a = avatar(&b).unwrap();
        let sh = *a.ring("shoulder").unwrap();
        assert!((sh.semi_axes().0 * 2.0 - (242.0 + 370.0 / PI)).abs() < 0.01, "{:?}", sh.semi_axes());
        let s = 410.0 * BACK_RISE / (WOMEN.cervicale_height - WOMEN.waist_height);
        assert!((a.neck_height - sh.z - (WOMEN.cervicale_height - WOMEN.acromial_height) * s).abs() < 1e-9);
        for (k, side) in [(0, 1.0), (1, -1.0)] {
            let arm = &a.arms[k];
            assert_eq!(
                arm.iter().map(|r| r.name).collect::<Vec<_>>(),
                ["armcap", "armtop", "upperarm", "elbow", "wrist", "palm", "fingers", "fingertips"]
            );
            // The upper arm round as measured, square to the arm (level across the tilted arm it
            // is longer by 1/cos).
            let (u, d) = arm[2].semi_axes();
            let square = ellipse_girth(u * ARM_ANGLE.cos(), d);
            assert!((square - 280.0).abs() < 280.0 * 0.03, "{square}");
            assert!(arm[2].girth > square);
            // Down the outside of the arm, shoulder to wrist: the sleeve length.
            let along = ((sh.z - arm[4].z).powi(2) + (arm[4].x - arm[2].x).powi(2)).sqrt();
            assert!((along - 590.0).abs() < 25.0, "{along}");
            // Out of the side of the chest at the armpit, the top of the arm round over the
            // shoulder point; at the wrist clear of the hips.
            let bust = a.ring("bust").unwrap().semi_axes().0;
            assert!(side * arm[2].x > bust && side * arm[2].x - u < bust, "{:?} {bust}", arm[2]);
            assert!(side * arm[1].x + arm[1].semi_axes().0 > sh.semi_axes().0, "{:?}", arm[1]);
            assert!(arm[0].z > arm[1].z && arm[0].girth < arm[1].girth);
            let hip = a.ring("hip").unwrap().semi_axes().0;
            assert!(side * arm[4].x - arm[4].semi_axes().0 > hip, "{:?}", arm[4]);
            assert!(arm.windows(2).all(|w| w[0].z > w[1].z) && side * arm[4].x > side * arm[1].x);
        }
        // Without them, from the survey's proportions.
        let plain = avatar(&body()).unwrap();
        assert!((plain.ring("shoulder").unwrap().semi_axes().0 * 2.0 - 880.0 * 0.386).abs() < 1e-9);
        let (u, d) = plain.arms[0][2].semi_axes();
        assert!((ellipse_girth(u * ARM_ANGLE.cos(), d) - 880.0 * 0.323).abs() < 880.0 * 0.323 * 0.03);
    }

    /// A male figure (PF-10.11) takes the survey's men's proportions: no underbust and no bust
    /// standing forward, the chest as deep as the men's, the shoulders and arms from theirs.
    #[test]
    fn a_male_figure_takes_the_mens_proportions() {
        let b = Body { bust: 1000.0, waist: 860.0, hip: 980.0, nape_to_waist: 460.0, waist_to_hip: 200.0, ..body() };
        let man = avatar(&Body { figure: Figure::Male, ..b }).unwrap();
        let woman = avatar(&b).unwrap();
        assert!(man.ring("underbust").is_none() && woman.ring("underbust").is_some());
        let chest = *man.ring("bust").unwrap();
        assert_eq!((chest.y, chest.girth, chest.depth_ratio), (0.0, 1000.0, MEN.chest_depth));
        // Every ring still as round as measured.
        assert_eq!(man.ring("waist").unwrap().girth, 860.0);
        assert_eq!(man.ring("hip").unwrap().depth_ratio, MEN.hip_depth);
        // Broader shoulders and a thicker upper arm for the same chest, from the men's shares.
        let breadth = |a: &Avatar| a.ring("shoulder").unwrap().semi_axes().0;
        assert!((breadth(&man) * 2.0 - 1000.0 * MEN.shoulders).abs() < 1e-9);
        assert!(breadth(&man) > breadth(&woman));
        // The same mesh, less the underbust's ring.
        assert_eq!(man.mesh.vertices.len(), woman.mesh.vertices.len() - RING_POINTS);
        assert_eq!(Figure::parse("male"), Some(Figure::Male));
        assert_eq!(Figure::parse(Figure::Female.name()), Some(Figure::Female));
        assert_eq!(Figure::parse("other"), None);
    }

    /// A child (PF-10.13): every band of the children's survey makes an avatar from its own mean
    /// measurements, its landmarks in order; between two bands the proportions are between theirs;
    /// no bust.
    #[test]
    fn a_child_figure_takes_the_childrens_proportions() {
        for (chest, sv) in &CHILD_BANDS {
            let hip = *chest * 1.04;
            let b = Body {
                bust: *chest,
                waist: *chest * 0.92,
                hip,
                nape_to_waist: (sv.cervicale_height - sv.waist_height) / BACK_RISE,
                waist_to_hip: sv.waist_height - sv.buttock_height,
                figure: Figure::Child,
                ..Default::default()
            };
            let a = avatar(&b).unwrap_or_else(|e| panic!("{chest}: {e}"));
            assert!(a.ring("underbust").is_none());
            assert_eq!(a.ring("bust").unwrap().depth_ratio, sv.chest_depth);
            // Its waist at the band's height (no waist to ankle given: the survey's).
            assert!((a.ring("waist").unwrap().z - sv.waist_height).abs() < 1e-6, "{chest}");
            assert!((a.legs[0][0].girth - hip * sv.thigh).abs() < 1e-9);
        }
        let (a, b) = (CHILD_BANDS[2], CHILD_BANDS[3]);
        let mid = child_survey((a.0 + b.0) / 2.0);
        assert!((mid.waist_height - (a.1.waist_height + b.1.waist_height) / 2.0).abs() < 1e-9);
        assert_eq!(child_survey(100.0), CHILD_BANDS[0].1);
        assert_eq!(child_survey(5000.0), CHILD_BANDS[CHILD_BANDS.len() - 1].1);
        assert_eq!(Figure::parse("child"), Some(Figure::Child));
    }

    /// A head and hands (PF-10.15): the head above the base of the neck as high as the survey's for
    /// the body's rise, as round as its share of the chest, longer front to back than broad; each
    /// hand on along the arm from the wrist, as long as the survey's share of the arm.
    #[test]
    fn the_crotch_is_as_long_as_the_surveys_share_of_the_hips() {
        // The test body: hips 960, the waist 200 above them; a man's too, and one with a crotch
        // depth given (its crotch lower, so drawn in more).
        let man = Body { figure: Figure::Male, ..body() };
        let deep = Body { crotch_depth: Some(280.0), ..body() };
        for (b, sv) in [(body(), WOMEN), (man, MEN), (deep, WOMEN)] {
            let a = avatar(&b).unwrap();
            let s = b.nape_to_waist * BACK_RISE / (sv.cervicale_height - sv.waist_height);
            let (waist, crotch) = (a.ring("waist").unwrap().z, a.ring("crotch").unwrap().z);
            let (front, back) = crotch_lengths(b.hip, waist - crotch, &sv, s);
            let (f, bk) = centre_lengths(&a).unwrap();
            assert!((f + bk - front - back).abs() < 1.0 && (f - front).abs() < 25.0, "{f} {bk} for {front} {back}");
            // 0.61 of the hips and a little more for the waist above the navel and for a deep
            // crotch: 600 to 780 here, where the hips carried straight down made it 755 to 795.
            assert!((600.0..780.0).contains(&(f + bk)), "{}", f + bk);
            // Drawn in from the hips' depth, front and back, the back the longer way.
            let (hip, crotch) = (a.ring("hip").unwrap(), a.ring("crotch").unwrap());
            assert!(crotch.semi_axes().1 < hip.semi_axes().1 * 0.8, "{:?} {:?}", crotch.semi_axes(), hip.semi_axes());
            assert!(bk > f);
            // The thighs side by side across the hips, just clear of each other, as round as the
            // survey's thigh.
            let (t0, t1) = (&a.legs[0][0], &a.legs[1][0]);
            let gap = (t0.x - t0.semi_axes().0) - (t1.x + t1.semi_axes().0);
            assert!((gap - THIGH_GAP).abs() < 1e-6, "{gap}");
            assert!((t0.x + t0.semi_axes().0 - (hip.semi_axes().0 - THIGH_GAP / 2.0)).abs() < 1e-6);
            assert!((t0.girth - b.hip * sv.thigh).abs() < 0.5, "{} {}", t0.girth, b.hip * sv.thigh);
        }
    }

    #[test]
    fn the_avatar_has_a_head_and_hands() {
        let a = avatar(&body()).unwrap();
        let head = *a.ring("head").unwrap();
        let rise = 410.0 * BACK_RISE;
        assert!((head.girth - 880.0 * WOMEN.head).abs() < 1e-9 && head.depth_ratio == WOMEN.head_shape);
        let top = a.torso[0];
        assert!((top.z - (a.neck_height + WOMEN.head_height * rise * 0.99)).abs() < 1e-9 && top.z > head.z && head.z > a.neck_height);
        let (b, l) = head.semi_axes();
        assert!(l > b, "longer than broad");
        // Still the neck's own height for the garments (the base of the neck).
        assert_eq!(a.ring("neck").unwrap().z, a.neck_height);
        for arm in &a.arms {
            let (wrist, tips) = (arm[4], arm[7]);
            let reach = ((tips.x - wrist.x).powi(2) + (tips.z - wrist.z).powi(2)).sqrt();
            let outseam = WOMEN.sleeve_outseam * rise / (WOMEN.cervicale_height - WOMEN.waist_height);
            assert!((reach - WOMEN.hand * outseam * 0.97).abs() < 1e-6, "{reach}");
            assert!(tips.z < wrist.z && tips.x.abs() > wrist.x.abs());
        }
        // A child's head is a bigger share of its chest than a woman's.
        let child =
            avatar(&Body { figure: Figure::Child, bust: 600.0, waist: 540.0, hip: 620.0, nape_to_waist: 280.0, waist_to_hip: 110.0, ..body() })
                .unwrap();
        assert!(child.ring("head").unwrap().girth / 600.0 > head.girth / 880.0);
    }

    /// Feet (PF-10.14): each leg ends in a foot as long as the survey's (its length over the ankle
    /// bone's height) and as broad, reaching forward of the ankle, low by the floor.
    #[test]
    fn each_leg_ends_in_a_foot() {
        let a = avatar(&body()).unwrap();
        for leg in &a.legs {
            let (ankle, foot) = (leg[leg.len() - 2], leg[leg.len() - 1]);
            assert_eq!((ankle.name, foot.name), ("ankle", "foot"));
            let (half_breadth, half_length) = foot.semi_axes();
            assert!((2.0 * half_length - ankle.z * WOMEN.foot).abs() < 1e-6);
            assert!((half_breadth / half_length - WOMEN.foot_breadth).abs() < 1e-9);
            assert!(foot.z > 0.0 && foot.z < ankle.z && foot.y > 0.0 && foot.x == ankle.x);
        }
    }

    /// The bust stands forward of a flat back: under it the ribcage as round as the underbust
    /// (else a B cup's 150 mm less than the bust), the bust's back in line with the ribcage's, its
    /// extra girth to the front.
    #[test]
    fn the_bust_stands_forward_of_a_flat_back() {
        for (under, expect) in [(None, 880.0 - 150.0), (Some(700.0), 700.0)] {
            let a = avatar(&Body { underbust: under, ..body() }).unwrap();
            let (bust, rib) = (*a.ring("bust").unwrap(), *a.ring("underbust").unwrap());
            assert_eq!(rib.girth, expect);
            assert!(rib.z < bust.z && rib.z > a.ring("waist").unwrap().z);
            let back = |r: &Ring| r.y - r.semi_axes().1;
            let front = |r: &Ring| r.y + r.semi_axes().1;
            assert!((back(&bust) - back(&rib)).abs() < 1e-9, "the back flat");
            assert!(front(&bust) > front(&rib) + 20.0, "the bust forward: {} {}", front(&bust), front(&rib));
            // The mesh's bust ring too.
            assert!(a.mesh.vertices.iter().any(|v| (v[2] - bust.z).abs() < 1e-9 && (v[1] - front(&bust)).abs() < 1e-6));
        }
    }

    #[test]
    fn the_mesh_is_closed_and_its_triangles_face_out() {
        let a = avatar(&body()).unwrap();
        let m = &a.mesh;
        // Five tubes, each its rings' points and two end points (the torso up through the head,
        // each leg down to its foot, each arm out to its fingertips).
        assert_eq!(m.vertices.len(), (6 + 7 + 5 + 5 + 8 + 8) * RING_POINTS + 10);
        // Closed: every edge is shared by exactly two triangles, once each way.
        let mut edges = std::collections::HashMap::new();
        for t in &m.triangles {
            for (p, q) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((p, q)).or_insert(0) += 1;
            }
        }
        assert!(edges.iter().all(|(&(p, q), &c)| c == 1 && edges.get(&(q, p)) == Some(&1)));
        // Facing out: the volume by the divergence theorem is positive.
        let v: f64 = m
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| m.vertices[i as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6.0
            })
            .sum();
        assert!(v > 0.0, "{v}");
        let obj = to_obj(m, "My body");
        assert!(obj.contains("\no My_body\n"));
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), m.vertices.len());
        assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), m.triangles.len());
    }
}
