//! Motion of composed models. Baking a model records its joints: the limbs of a figure, the
//! legs, neck and tail of a beast, wheels, oars, a floating hull, a part that swings when its
//! unit attacks. Each joint turns about a pivot in the model frame and carries its children.
//! [`Rig::pose`] gives every joint's transform for an [`Activity`] at a point of its cycle;
//! the GPU reads those poses from keyframes ([`Rig::keyframes`]) baked before play, picking the
//! cycle point from the render clock and a phase per entity. Presentation only: nothing here
//! reaches the simulation.

use std::f32::consts::{PI, TAU};

use crate::kit::Rotation;

/// Most joints a model may have, the model itself (joint 0) included.
pub const MAX_JOINTS: usize = 40;
/// Keyframes per activity cycle.
pub const POSE_SAMPLES: usize = 24;
/// Default cycle lengths in seconds: idle, move, attack, work.
const DEFAULT_PERIODS: [f32; 4] = [3.2, 1.0, 1.2, 1.2];
/// Point of the attack cycle at which a firearm or a gun fires: the muzzle flashes, a barrel
/// recoils and the shooter's piece kicks.
pub const FIRE_AT: f32 = 0.55;
/// Scale of a muzzle flash while it is not shown (a point, so it draws nothing).
const HIDDEN: f32 = 1e-3;

/// What a unit is doing, as far as its motion shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Activity {
    /// Standing (breathing).
    #[default]
    Idle,
    /// Walking, riding, rolling or rowing.
    Move,
    /// Fighting.
    Attack,
    /// Building, gathering or healing.
    Work,
}

impl Activity {
    /// Every activity, in the order of the keyframe rows.
    pub const ALL: [Activity; 4] = [
        Activity::Idle,
        Activity::Move,
        Activity::Attack,
        Activity::Work,
    ];

    /// Row of the activity in the keyframes.
    pub fn index(self) -> usize {
        self as usize
    }

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Activity::Idle => "idle",
            Activity::Move => "move",
            Activity::Attack => "attack",
            Activity::Work => "work",
        }
    }

    /// The activity a command-line name gives.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }
}

/// How a figure's right arm attacks (`figure` knob `attack`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackStyle {
    /// Overhead chop (tools, staffs, clubs).
    Strike,
    /// Underarm thrust, the weapon levelled forward.
    Thrust,
    /// Overhand throw, the weapon cocked behind the head.
    Throw,
    /// Draw and release of a bow held in the left hand.
    Draw,
    /// A sling whirled overhead, then released.
    Whirl,
    /// A long firearm or a crossbow: raised to the shoulder with both hands, aimed, fired,
    /// lowered.
    Fire,
    /// A pistol: the right arm levels it, fires, lowers it.
    Pistol,
}

/// How a figure works (`figure` knob `work`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkStyle {
    /// A tool swung with both arms.
    Tool,
    /// Both arms raised (blessing, healing).
    Raise,
}

/// The knobs of a figure that shape its motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigureStyle {
    /// Height in model units (the figure's `h`).
    pub height: f32,
    /// Attack style.
    pub attack: AttackStyle,
    /// Work style.
    pub work: WorkStyle,
    /// Arms carried still this far forward (radians), right then left; 0 swings freely.
    pub hold: [f32; 2],
    /// Leg swing of the walk, radians.
    pub stride: f32,
    /// Sitting astride a mount: the legs stay still.
    pub seated: bool,
    /// A long robe: short steps.
    pub robe: bool,
}

/// The knobs of a beast that shape its gait.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeastStyle {
    /// Size factor in model units (the beast's `h`).
    pub height: f32,
    /// Leg swing of the gait, radians.
    pub stride: f32,
}

/// What moves a joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Follows its parent only (joint 0, the model).
    Fixed,
    /// A figure's body: bob, breath and lean.
    Body(FigureStyle),
    /// A figure's leg; side 0 is the right (−x), 1 the left.
    Leg(FigureStyle, usize),
    /// A figure's arm, from the shoulder.
    Arm(FigureStyle, usize),
    /// What a figure holds in that hand, turning in the hand.
    Grip(FigureStyle, usize),
    /// A beast's body.
    Beast(BeastStyle),
    /// A beast's leg: front or back, side 0 right (−x) or 1 left.
    BeastLeg(BeastStyle, bool, usize),
    /// A beast's neck and head.
    Neck,
    /// A beast's tail.
    Tail,
    /// A wheel turning about its axle (the part's x axis) while moving.
    Wheel,
    /// An oar of the bank's side `+1` (+z) or `-1` (−z), sweeping while moving.
    Oar(f32),
    /// A hull riding the water: bob, roll and pitch; under way it heels this many radians
    /// (a sailing ship leaning to the wind).
    Float(f32),
    /// A part swinging forward about its x axis by this many radians when attacking.
    Strike(f32),
    /// A gun barrel kicking back this far (model units) along its own −y when it fires.
    Recoil(f32),
    /// A muzzle flash: shown only for a moment when its unit fires.
    Flash,
}

impl Motion {
    /// Whether the joint moves its pivot (a bob, a breath, a recoil); every other joint turns
    /// about a pivot that stays where its parent carries it.
    pub fn moves_pivot(&self) -> bool {
        matches!(
            self,
            Motion::Body(_) | Motion::Beast(_) | Motion::Float(_) | Motion::Recoil(_)
        )
    }
}

/// One joint: a pivot and frame in the model, its parent and its motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    /// Index of the parent joint (0: the model).
    pub parent: u8,
    /// The pivot, in the model frame at rest.
    pub pivot: [f32; 3],
    /// The joint's axes in the model frame, as the columns of a rotation.
    pub frame: Rotation,
    /// Model units per unit of the joint's own frame (an include's scale).
    pub scale: f32,
    /// What moves it.
    pub motion: Motion,
}

/// A 3 by 4 affine transform: rotation in the first three columns, translation in the last.
pub type Affine = [[f32; 4]; 3];

/// The transform that changes nothing.
pub const IDENTITY: Affine = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// `a` after `b`.
pub fn compose(a: &Affine, b: &Affine) -> Affine {
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let r = (0..3).map(|k| a[i][k] * b[k][j]).sum::<f32>();
            if j == 3 { r + a[i][3] } else { r }
        })
    })
}

/// The point `p` moved by `a`.
pub fn apply(a: &Affine, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i][0] * p[0] + a[i][1] * p[1] + a[i][2] * p[2] + a[i][3])
}

fn mul(a: &Rotation, b: &Rotation) -> Rotation {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn transpose(a: &Rotation) -> Rotation {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}

fn turn(a: &Rotation, v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i][0] * v[0] + a[i][1] * v[1] + a[i][2] * v[2])
}

const ROT_IDENTITY: Rotation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Turn about x by `a` radians (y towards z).
fn rx(a: f32) -> Rotation {
    let (s, c) = a.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

/// Turn about y by `a` radians (z towards x).
fn ry(a: f32) -> Rotation {
    let (s, c) = a.sin_cos();
    [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
}

/// Turn about z by `a` radians (x towards y).
fn rz(a: f32) -> Rotation {
    let (s, c) = a.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

/// A hanging limb swung forward (towards +z) by `a` radians.
fn swing(a: f32) -> Rotation {
    rx(-a)
}

fn deg(d: f32) -> f32 {
    d.to_radians()
}

/// A value along the cycle `u` (0..1): `rest` at both ends, through `keys` (cycle point, value),
/// eased between keys.
fn track(u: f32, rest: f32, keys: &[(f32, f32)]) -> f32 {
    let mut from = (0.0, rest);
    for &key in keys.iter().chain(std::iter::once(&(1.0, rest))) {
        if u <= key.0 {
            let span = (key.0 - from.0).max(1e-6);
            let t = ((u - from.0) / span).clamp(0.0, 1.0);
            let t = t * t * (3.0 - 2.0 * t);
            return from.1 + (key.1 - from.1) * t;
        }
        from = key;
    }
    rest
}

/// Pose of a figure's parts at one point of a cycle (figure units: height 1).
struct FigurePose {
    bob: f32,
    lean: f32,
    /// Turn of the body about the vertical, radians (positive turns the right shoulder
    /// forward).
    yaw: f32,
    legs: [Rotation; 2],
    arms: [Rotation; 2],
    grips: [Rotation; 2],
}

fn figure_pose(style: &FigureStyle, activity: Activity, u: f32) -> FigurePose {
    let phase = TAU * u;
    let hold = style.hold;
    let held = |side: usize| hold[side] != 0.0;
    // A held arm keeps what it carries upright; a free one swings it along.
    let carry = |side: usize, angle: f32| {
        if held(side) { rx(angle) } else { ROT_IDENTITY }
    };
    let mut pose = FigurePose {
        bob: 0.0,
        lean: 0.0,
        yaw: 0.0,
        legs: [ROT_IDENTITY; 2],
        arms: [swing(hold[0]), swing(hold[1])],
        grips: [carry(0, hold[0]), carry(1, hold[1])],
    };
    match activity {
        Activity::Idle => {
            pose.bob = 0.005 * phase.sin();
            for (side, held) in hold.iter().enumerate() {
                let a = held + deg(2.0) * (phase + side as f32 * PI).sin();
                pose.arms[side] = swing(a);
                pose.grips[side] = carry(side, a);
            }
        }
        Activity::Move => {
            let c = phase.cos();
            let stride = if style.seated {
                0.0
            } else if style.robe {
                style.stride * 0.55
            } else {
                style.stride
            };
            pose.legs = [swing(stride * c), swing(-stride * c)];
            // The hips drop as the legs spread, so the feet stay on the ground.
            pose.bob = -0.4 * (1.0 - (stride * c).cos());
            pose.lean = if style.seated { 0.06 } else { 0.07 };
            let arm = if style.seated { deg(6.0) } else { stride * 0.8 };
            for (side, sign) in [(0, -1.0), (1, 1.0)] {
                let a = if held(side) {
                    hold[side] + 0.1 * arm * sign * c
                } else {
                    sign * arm * c
                };
                pose.arms[side] = swing(a);
                pose.grips[side] = carry(side, a);
            }
        }
        Activity::Attack => attack_pose(style, u, &mut pose),
        Activity::Work => work_pose(style, u, &mut pose),
    }
    // The body leans about the hips; the legs, hanging from the same line, stay upright. A
    // standing figure turns on its feet; a rider's legs keep straddling the mount.
    let upright = if style.seated {
        transpose(&mul(&ry(pose.yaw), &rx(pose.lean)))
    } else {
        rx(-pose.lean)
    };
    pose.legs = pose.legs.map(|leg| mul(&upright, &leg));
    pose
}

fn work_pose(style: &FigureStyle, u: f32, pose: &mut FigurePose) {
    let hold = style.hold;
    let carry = |side: usize, angle: f32| {
        if hold[side] != 0.0 {
            rx(angle)
        } else {
            ROT_IDENTITY
        }
    };
    match style.work {
        WorkStyle::Tool => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.35, deg(140.0)),
                    (0.45, deg(148.0)),
                    (0.6, deg(25.0)),
                    (0.75, deg(20.0)),
                ],
            );
            let l = track(u, hold[1], &[(0.35, deg(70.0)), (0.6, deg(30.0))]);
            pose.arms = [swing(r), swing(l)];
            pose.grips = [carry(0, r), carry(1, l)];
            pose.lean = track(u, 0.0, &[(0.35, -0.04), (0.6, 0.22), (0.75, 0.2)]);
        }
        WorkStyle::Raise => {
            let r = track(u, hold[0], &[(0.3, deg(150.0)), (0.7, deg(150.0))]);
            let l = track(u, hold[1], &[(0.3, deg(150.0)), (0.7, deg(150.0))]);
            pose.arms = [swing(r), swing(l)];
            pose.grips = [carry(0, r), carry(1, l)];
            pose.lean = track(u, 0.0, &[(0.3, -0.06), (0.7, -0.06)]);
        }
    }
}

/// The hips and the right shoulder of a figure one unit tall and of build 1 (the left
/// shoulder mirrors it), and the reach from a shoulder to the hand: what an aim is worked out
/// on.
const HIP: [f32; 3] = [0.0, 0.4, 0.0];
const SHOULDER: [f32; 3] = [-0.205, 0.72, 0.0];
const REACH: f32 = 0.27;
/// Turn of a shooter's body while a long gun is up: the left shoulder towards the target.
const AIM_TURN: f32 = -50.0 * PI / 180.0;
/// Lean of a shooter's body into the aim.
const AIM_LEAN: f32 = 0.02;
/// Where the right hand holds a shouldered long gun (figure frame, x and height): on the
/// stock beside the cheek, the butt in the shoulder.
const STOCK_HAND: [f32; 2] = [-0.09, 0.77];
/// Where the left hand holds it: under the barrel, ahead of the right.
const BARREL_HAND: [f32; 2] = [-0.075, 0.74];

/// The swing and turn of the arm on `side` (0 right, 1 left) that bring its hand to `hand`
/// (x and height in the figure frame; the hand reaches as far forward as the arm allows) while
/// the body is turned by `body`; the arm's rotation is `ry(turn) · swing(swing)`.
fn reach(body: &Rotation, side: usize, hand: [f32; 2]) -> (f32, f32) {
    let x = if side == 0 { SHOULDER[0] } else { -SHOULDER[0] };
    let rest = [x - HIP[0], SHOULDER[1] - HIP[1], SHOULDER[2] - HIP[2]];
    let turned = turn(body, rest);
    let shoulder: [f32; 3] = std::array::from_fn(|i| HIP[i] + turned[i]);
    let (dx, dy) = (hand[0] - shoulder[0], hand[1] - shoulder[1]);
    let dz = (REACH * REACH - dx * dx - dy * dy).max(0.0).sqrt();
    let local = turn(&transpose(body), [dx, dy, dz]);
    let length = local.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
    let swing = (-local[1] / length).clamp(-1.0, 1.0).acos();
    (swing, local[0].atan2(local[2]))
}

fn attack_pose(style: &FigureStyle, u: f32, pose: &mut FigurePose) {
    let hold = style.hold;
    match style.attack {
        AttackStyle::Strike => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.4, deg(165.0)),
                    (0.52, deg(172.0)),
                    (0.66, deg(35.0)),
                    (0.8, deg(30.0)),
                ],
            );
            pose.arms[0] = swing(r);
            pose.grips[0] = if hold[0] != 0.0 { rx(r) } else { ROT_IDENTITY };
            pose.lean = track(u, 0.0, &[(0.4, -0.06), (0.66, 0.16), (0.8, 0.12)]);
        }
        AttackStyle::Thrust => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.2, deg(40.0)),
                    (0.42, deg(25.0)),
                    (0.56, deg(75.0)),
                    (0.7, deg(70.0)),
                    (0.85, deg(40.0)),
                ],
            );
            // The weapon levels to point forward, whatever the arm does.
            let level = track(u, 0.0, &[(0.2, deg(85.0)), (0.85, deg(85.0))]);
            pose.arms[0] = swing(r);
            pose.grips[0] = rx(r + level);
            pose.lean = track(u, 0.0, &[(0.42, -0.04), (0.56, 0.14), (0.75, 0.1)]);
        }
        AttackStyle::Throw => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.3, deg(200.0)),
                    (0.45, deg(206.0)),
                    (0.58, deg(95.0)),
                    (0.7, deg(55.0)),
                ],
            );
            let aim = track(
                u,
                0.0,
                &[(0.25, deg(70.0)), (0.6, deg(80.0)), (0.75, deg(60.0))],
            );
            pose.arms[0] = swing(r);
            pose.grips[0] = rx(r + aim);
            pose.lean = track(u, 0.0, &[(0.45, -0.1), (0.6, 0.15)]);
        }
        AttackStyle::Draw => {
            let l = track(u, hold[1], &[(0.12, deg(85.0)), (0.88, deg(85.0))]);
            pose.arms[1] = swing(l);
            // The bow stays upright in the outstretched hand.
            pose.grips[1] = rx(l);
            let r = track(u, hold[0], &[(0.12, deg(85.0)), (0.88, deg(85.0))]);
            let pull = track(
                u,
                0.0,
                &[
                    (0.15, 0.0),
                    (0.6, deg(-75.0)),
                    (0.76, deg(-75.0)),
                    (0.8, deg(-5.0)),
                    (0.88, 0.0),
                ],
            );
            pose.arms[0] = mul(&ry(pull), &swing(r));
            pose.grips[0] = ROT_IDENTITY;
        }
        AttackStyle::Whirl => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.12, deg(130.0)),
                    (0.62, deg(130.0)),
                    (0.74, deg(70.0)),
                    (0.86, deg(45.0)),
                ],
            );
            let spin = if (0.12..0.62).contains(&u) {
                2.0 * TAU * (u - 0.12) / 0.5
            } else {
                0.0
            };
            pose.arms[0] = mul(&ry(spin), &swing(r));
            pose.grips[0] = ROT_IDENTITY;
            pose.lean = track(u, 0.0, &[(0.62, -0.05), (0.74, 0.12)]);
        }
        AttackStyle::Fire => {
            // Up to the shoulder, aim, the kick of the shot, back down to the carry. The
            // shooter turns the left shoulder towards the target, beds the butt in the right
            // shoulder with the piece beside the cheek and holds the barrel up with the left
            // hand.
            let t = track(u, 0.0, &[(0.22, 1.0), (0.74, 1.0), (0.9, 0.0)]);
            let aimed = mul(&ry(AIM_TURN), &rx(AIM_LEAN));
            let (ra, rb) = reach(&aimed, 0, STOCK_HAND);
            let (la, lb) = reach(&aimed, 1, BARREL_HAND);
            pose.yaw = AIM_TURN * t;
            pose.lean = track(
                u,
                0.0,
                &[
                    (0.22, AIM_LEAN),
                    (FIRE_AT, AIM_LEAN),
                    (FIRE_AT + 0.05, -0.05),
                    (0.74, AIM_LEAN),
                    (0.9, 0.0),
                ],
            );
            let body = mul(&ry(pose.yaw), &rx(pose.lean));
            pose.arms[0] = mul(&ry(rb * t), &swing(hold[0] + (ra - hold[0]) * t));
            pose.arms[1] = mul(&ry(lb * t), &swing(hold[1] + (la - hold[1]) * t));
            // The piece points forward while aimed, its muzzle jumping up as it fires,
            // whatever the body and arms do.
            let aim = track(
                u,
                0.0,
                &[
                    (0.22, deg(90.0)),
                    (FIRE_AT, deg(90.0)),
                    (FIRE_AT + 0.05, deg(74.0)),
                    (0.74, deg(90.0)),
                    (0.9, 0.0),
                ],
            );
            let carried = |side: usize| transpose(&mul(&body, &pose.arms[side]));
            pose.grips[0] = mul(&carried(0), &rx(aim));
            pose.grips[1] = if hold[1] != 0.0 {
                carried(1)
            } else {
                ROT_IDENTITY
            };
        }
        AttackStyle::Pistol => {
            let r = track(
                u,
                hold[0],
                &[
                    (0.25, deg(86.0)),
                    (FIRE_AT, deg(86.0)),
                    (FIRE_AT + 0.05, deg(104.0)),
                    (0.74, deg(86.0)),
                    (0.9, hold[0]),
                ],
            );
            let aim = track(
                u,
                0.0,
                &[
                    (0.25, deg(90.0)),
                    (FIRE_AT, deg(90.0)),
                    (FIRE_AT + 0.05, deg(70.0)),
                    (0.74, deg(90.0)),
                    (0.9, 0.0),
                ],
            );
            pose.arms[0] = swing(r);
            pose.grips[0] = rx(r + aim);
            pose.lean = track(u, 0.0, &[(FIRE_AT, 0.03), (FIRE_AT + 0.05, -0.04)]);
        }
    }
}

/// Size of a muzzle flash at the point `u` of the attack cycle: a point, then full size for a
/// moment just after [`FIRE_AT`].
fn flash_scale(u: f32) -> f32 {
    track(
        u,
        HIDDEN,
        &[
            (FIRE_AT, HIDDEN),
            (FIRE_AT + 0.02, 1.0),
            (FIRE_AT + 0.07, 1.0),
            (FIRE_AT + 0.11, HIDDEN),
        ],
    )
}

/// Rotation and translation (joint frame, joint units) of a beast's joint.
fn beast_pose(
    style: &BeastStyle,
    joint: &Motion,
    activity: Activity,
    u: f32,
) -> (Rotation, [f32; 3]) {
    let phase = TAU * u;
    let stride = style.stride;
    match (*joint, activity) {
        (Motion::Beast(_), Activity::Move) => {
            let c = phase.cos();
            let bob = -0.5 * (1.0 - (stride * c).cos()) - 0.02 * (2.0 * phase).sin().abs();
            (rx(0.035 * phase.sin()), [0.0, bob * style.height, 0.0])
        }
        (Motion::Beast(_), _) => (ROT_IDENTITY, [0.0, 0.004 * phase.sin() * style.height, 0.0]),
        (Motion::BeastLeg(_, front, side), Activity::Move) => {
            // Diagonal pairs move together: front right with back left.
            let pair = if front == (side == 0) { 1.0 } else { -1.0 };
            let reach = if front { 1.0 } else { 0.85 };
            (swing(pair * reach * stride * phase.cos()), [0.0; 3])
        }
        (Motion::BeastLeg(_, true, 0), Activity::Attack) => {
            (swing(0.45 * (PI * u).sin().powi(2)), [0.0; 3])
        }
        _ => (ROT_IDENTITY, [0.0; 3]),
    }
}

/// Rotation and translation (in the joint's frame and units) of one joint.
fn local(motion: &Motion, activity: Activity, u: f32) -> (Rotation, [f32; 3]) {
    let phase = TAU * u;
    match motion {
        Motion::Fixed => (ROT_IDENTITY, [0.0; 3]),
        Motion::Body(style) => {
            let pose = figure_pose(style, activity, u);
            (
                mul(&ry(pose.yaw), &rx(pose.lean)),
                [0.0, pose.bob * style.height, 0.0],
            )
        }
        Motion::Leg(style, side) => (figure_pose(style, activity, u).legs[*side], [0.0; 3]),
        Motion::Arm(style, side) => (figure_pose(style, activity, u).arms[*side], [0.0; 3]),
        Motion::Grip(style, side) => (figure_pose(style, activity, u).grips[*side], [0.0; 3]),
        Motion::Beast(style) | Motion::BeastLeg(style, _, _) => {
            beast_pose(style, motion, activity, u)
        }
        Motion::Neck => match activity {
            Activity::Move => (rx(0.07 * (2.0 * phase).sin()), [0.0; 3]),
            Activity::Attack => (rx(-0.18 * (PI * u).sin()), [0.0; 3]),
            _ => (rx(0.14 * (1.0 - phase.cos()) / 2.0), [0.0; 3]),
        },
        Motion::Tail => match activity {
            Activity::Move => (mul(&rx(-0.3), &rz(0.12 * phase.sin())), [0.0; 3]),
            _ => (rz(0.18 * phase.sin()), [0.0; 3]),
        },
        Motion::Wheel => match activity {
            Activity::Move => (rx(TAU * u), [0.0; 3]),
            _ => (ROT_IDENTITY, [0.0; 3]),
        },
        Motion::Oar(side) => match activity {
            Activity::Move | Activity::Attack => {
                // Blades dip at the bow, pull to the stern in the water and come back above it.
                let sweep = side * deg(22.0) * phase.cos();
                let dip = side * deg(9.0) * phase.sin();
                (mul(&ry(sweep), &rx(dip)), [0.0; 3])
            }
            _ => (rx(-side * deg(5.0)), [0.0; 3]),
        },
        Motion::Float(heel) => {
            let lean = if activity == Activity::Move {
                *heel
            } else {
                0.0
            };
            let roll = lean + deg(1.8) * (phase + 0.6).sin();
            let pitch = deg(0.8) * (phase + 2.1).sin();
            (mul(&rx(roll), &rz(pitch)), [0.0, 0.02 * phase.sin(), 0.0])
        }
        Motion::Strike(angle) => match activity {
            Activity::Attack => {
                let a = track(u, 0.0, &[(0.12, *angle), (0.3, *angle)]);
                (rx(a), [0.0; 3])
            }
            _ => (ROT_IDENTITY, [0.0; 3]),
        },
        Motion::Recoil(distance) => match activity {
            Activity::Attack => {
                let back = track(
                    u,
                    0.0,
                    &[(FIRE_AT, 0.0), (FIRE_AT + 0.03, -*distance), (0.92, 0.0)],
                );
                (ROT_IDENTITY, [0.0, back, 0.0])
            }
            _ => (ROT_IDENTITY, [0.0; 3]),
        },
        Motion::Flash => {
            let s = if activity == Activity::Attack {
                flash_scale(u)
            } else {
                HIDDEN
            };
            ([[s, 0.0, 0.0], [0.0, s, 0.0], [0.0, 0.0, s]], [0.0; 3])
        }
    }
}

/// The joints of a model and the length of each activity's cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    /// The joints; joint 0 is the model itself and every parent comes before its children.
    pub joints: Vec<Joint>,
    /// Cycle lengths in seconds, by [`Activity::index`]; 0 until a part sets one.
    periods: [f32; 4],
    /// Whether a joint did not fit.
    overflow: bool,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            joints: vec![Joint {
                parent: 0,
                pivot: [0.0; 3],
                frame: ROT_IDENTITY,
                scale: 1.0,
                motion: Motion::Fixed,
            }],
            periods: [0.0; 4],
            overflow: false,
        }
    }
}

impl Rig {
    /// Whether any part moves.
    pub fn is_animated(&self) -> bool {
        self.joints.len() > 1
    }

    /// Whether a joint was refused because the rig was full.
    pub fn overflowed(&self) -> bool {
        self.overflow
    }

    /// Adds a joint; `None` when the rig is full (or the parent unknown).
    pub fn add(&mut self, joint: Joint) -> Option<u8> {
        if self.joints.len() >= MAX_JOINTS || usize::from(joint.parent) >= self.joints.len() {
            self.overflow = true;
            return None;
        }
        self.joints.push(joint);
        u8::try_from(self.joints.len() - 1).ok()
    }

    /// Sets the cycle of `activity` to `seconds` unless an earlier part set it.
    pub fn suggest_period(&mut self, activity: Activity, seconds: f32) {
        let slot = &mut self.periods[activity.index()];
        if *slot <= 0.0 && seconds > 0.0 {
            *slot = seconds;
        }
    }

    /// Cycle length of `activity`, seconds.
    pub fn period(&self, activity: Activity) -> f32 {
        let set = self.periods[activity.index()];
        if set > 0.0 {
            set
        } else {
            DEFAULT_PERIODS[activity.index()]
        }
    }

    /// Every joint's transform (model frame at rest to model frame posed) at the point `u`
    /// (0..1) of the cycle of `activity`.
    pub fn pose(&self, activity: Activity, u: f32) -> Vec<Affine> {
        let u = u.rem_euclid(1.0);
        let mut out: Vec<Affine> = Vec::with_capacity(self.joints.len());
        for (index, joint) in self.joints.iter().enumerate() {
            if index == 0 {
                out.push(IDENTITY);
                continue;
            }
            let (r, t) = local(&joint.motion, activity, u);
            let f = &joint.frame;
            let m = mul(&mul(f, &r), &transpose(f));
            let shift = turn(f, t.map(|v| v * joint.scale));
            let turned = turn(&m, joint.pivot);
            let own: Affine = std::array::from_fn(|i| {
                [
                    m[i][0],
                    m[i][1],
                    m[i][2],
                    joint.pivot[i] - turned[i] + shift[i],
                ]
            });
            let parent = out[usize::from(joint.parent).min(index - 1)];
            out.push(compose(&parent, &own));
        }
        out
    }

    /// The keyframes as RGBA texels, `3 × joints` wide and `4 × POSE_SAMPLES` high: row
    /// `activity × POSE_SAMPLES + sample` holds each joint's transform as three rows of four.
    pub fn keyframes(&self) -> Vec<[f32; 4]> {
        let mut texels = Vec::with_capacity(self.joints.len() * 3 * 4 * POSE_SAMPLES);
        for activity in Activity::ALL {
            for sample in 0..POSE_SAMPLES {
                let pose = self.pose(activity, sample as f32 / POSE_SAMPLES as f32);
                for transform in &pose {
                    texels.extend(transform.iter().copied());
                }
            }
        }
        texels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> FigureStyle {
        FigureStyle {
            height: 1.0,
            attack: AttackStyle::Strike,
            work: WorkStyle::Tool,
            hold: [0.0; 2],
            stride: deg(28.0),
            seated: false,
            robe: false,
        }
    }

    /// A figure's body, a leg and an arm with a grip below it.
    fn figure(style: FigureStyle) -> Rig {
        let mut rig = Rig::default();
        let joint = |parent: u8, pivot: [f32; 3], motion: Motion| Joint {
            parent,
            pivot,
            frame: ROT_IDENTITY,
            scale: 1.0,
            motion,
        };
        let body = rig
            .add(joint(0, [0.0, 0.4, 0.0], Motion::Body(style)))
            .unwrap();
        rig.add(joint(body, [-0.06, 0.4, 0.0], Motion::Leg(style, 0)))
            .unwrap();
        let arm = rig
            .add(joint(body, [-0.2, 0.72, 0.0], Motion::Arm(style, 0)))
            .unwrap();
        rig.add(joint(arm, [-0.2, 0.45, 0.03], Motion::Grip(style, 0)))
            .unwrap();
        rig
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn joints_stay_on_their_parents() {
        for attack in [
            AttackStyle::Strike,
            AttackStyle::Thrust,
            AttackStyle::Throw,
            AttackStyle::Draw,
            AttackStyle::Whirl,
            AttackStyle::Fire,
            AttackStyle::Pistol,
        ] {
            let rig = figure(FigureStyle { attack, ..style() });
            for activity in Activity::ALL {
                for s in 0..POSE_SAMPLES {
                    let pose = rig.pose(activity, s as f32 / POSE_SAMPLES as f32);
                    for (i, joint) in rig.joints.iter().enumerate().skip(1) {
                        if joint.motion.moves_pivot() {
                            continue;
                        }
                        let own = apply(&pose[i], joint.pivot);
                        let parent = apply(&pose[usize::from(joint.parent)], joint.pivot);
                        assert!(close(own, parent), "{attack:?} {activity:?} {s}: joint {i}");
                    }
                }
            }
        }
    }

    #[test]
    fn cycles_close_on_themselves() {
        let rig = figure(style());
        for activity in Activity::ALL {
            let (a, b) = (rig.pose(activity, 0.0), rig.pose(activity, 1.0 - 1e-6));
            for (x, y) in a.iter().zip(&b) {
                for i in 0..3 {
                    for j in 0..4 {
                        assert!((x[i][j] - y[i][j]).abs() < 1e-3, "{activity:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_walk_moves_the_legs_and_keeps_the_feet_down() {
        let rig = figure(style());
        let foot = [-0.06, 0.0, 0.0];
        let at = |u: f32| apply(&rig.pose(Activity::Move, u)[2], foot);
        // A quarter cycle apart the leg is forward, under the body, then back.
        let (a, b, c) = (at(0.0), at(0.25), at(0.5));
        assert!(
            a[2] > 0.1 && c[2] < -0.1 && b[2].abs() < 0.02,
            "{a:?} {b:?} {c:?}"
        );
        for u in [0.0, 0.1, 0.25, 0.4] {
            assert!(
                at(u)[1].abs() < 0.01,
                "foot off the ground at {u}: {:?}",
                at(u)
            );
        }
        // Idle hardly moves.
        let idle = apply(&rig.pose(Activity::Idle, 0.3)[2], foot);
        assert!(close([idle[0], 0.0, idle[2]], [-0.06, 0.0, 0.0]));
    }

    #[test]
    fn a_thrust_levels_the_weapon() {
        let rig = figure(FigureStyle {
            attack: AttackStyle::Thrust,
            ..style()
        });
        let pose = rig.pose(Activity::Attack, 0.56);
        // The weapon's own up axis points forward.
        let grip = &pose[4];
        let up = [grip[0][1], grip[1][1], grip[2][1]];
        assert!(up[2] > 0.9, "{up:?}");
    }

    /// The weapon's own up axis (model frame) at `u` of the attack of a figure attacking so.
    fn weapon_up(attack: AttackStyle, u: f32) -> [f32; 3] {
        let rig = figure(FigureStyle {
            attack,
            hold: [deg(14.0), 0.0],
            ..style()
        });
        let grip = &rig.pose(Activity::Attack, u)[4];
        [grip[0][1], grip[1][1], grip[2][1]]
    }

    #[test]
    fn a_firearm_is_raised_aimed_fired_and_lowered() {
        for attack in [AttackStyle::Fire, AttackStyle::Pistol] {
            // Carried upright, levelled forward while aimed, the muzzle jumping up at the shot.
            let carried = weapon_up(attack, 0.0);
            assert!(carried[1] > 0.99, "{attack:?} {carried:?}");
            let aimed = weapon_up(attack, 0.4);
            assert!(aimed[2] > 0.99, "{attack:?} {aimed:?}");
            let kick = weapon_up(attack, FIRE_AT + 0.05);
            assert!(kick[1] > 0.2 && kick[2] > 0.9, "{attack:?} {kick:?}");
            let lowered = weapon_up(attack, 0.95);
            assert!(lowered[1] > 0.99, "{attack:?} {lowered:?}");
        }
    }

    #[test]
    fn a_long_gun_is_fired_from_the_shoulder_with_both_hands() {
        let style = FigureStyle {
            attack: AttackStyle::Fire,
            hold: [deg(14.0), 0.0],
            ..style()
        };
        // A figure with both arms and a grip in each hand, as the kit builds it.
        let mut rig = Rig::default();
        let joint = |parent: u8, pivot: [f32; 3], motion: Motion| Joint {
            parent,
            pivot,
            frame: ROT_IDENTITY,
            scale: 1.0,
            motion,
        };
        let body = rig.add(joint(0, HIP, Motion::Body(style))).unwrap();
        let hands = [0_usize, 1].map(|side| {
            let x = if side == 0 { SHOULDER[0] } else { -SHOULDER[0] };
            let arm = rig
                .add(joint(body, [x, 0.72, 0.0], Motion::Arm(style, side)))
                .unwrap();
            rig.add(joint(arm, [x, 0.45, 0.03], Motion::Grip(style, side)))
                .unwrap()
        });
        let pose = rig.pose(Activity::Attack, 0.4);
        let at = |side: usize| {
            let j = usize::from(hands[side]);
            apply(&pose[j], rig.joints[j].pivot)
        };
        let (right, left) = (at(0), at(1));
        let grip = &pose[usize::from(hands[0])];
        let along = [grip[0][1], grip[1][1], grip[2][1]];
        assert!(along[2] > 0.99, "aimed forward: {along:?}");
        // The right hand is up by the cheek, beside the head and near the shoulder line.
        assert!(
            right[1] > 0.7 && right[0] < -0.03 && right[0] > -0.16,
            "{right:?}"
        );
        // The left hand is ahead of it, under the same line of the barrel.
        assert!(left[2] > right[2] + 0.12, "{left:?} {right:?}");
        assert!((left[0] - right[0]).abs() < 0.05, "{left:?} {right:?}");
        assert!(
            left[1] < right[1] && right[1] - left[1] < 0.08,
            "{left:?} {right:?}"
        );
        // Carried at the start and the end of the cycle: the body faces forward.
        for u in [0.0, 0.95] {
            let rest = rig.pose(Activity::Attack, u);
            let facing = [rest[1][0][2], rest[1][2][2]];
            assert!(facing[1] > 0.99, "{u}: {facing:?}");
        }
    }

    #[test]
    fn a_turning_rider_keeps_straddling_the_mount() {
        let rig = figure(FigureStyle {
            attack: AttackStyle::Fire,
            seated: true,
            ..style()
        });
        let pose = rig.pose(Activity::Attack, 0.4);
        // The body is turned; the leg (joint 2) keeps the model's axes.
        assert!(pose[1][0][0] < 0.9, "{:?}", pose[1]);
        for i in 0..3 {
            assert!((pose[2][i][i] - 1.0).abs() < 1e-4, "{:?}", pose[2]);
        }
    }

    /// A rig with one joint of `motion` at `pivot`.
    fn single(motion: Motion, pivot: [f32; 3]) -> Rig {
        let mut rig = Rig::default();
        rig.add(Joint {
            parent: 0,
            pivot,
            frame: ROT_IDENTITY,
            scale: 1.0,
            motion,
        })
        .unwrap();
        rig
    }

    #[test]
    fn a_flash_shows_only_as_the_shot_goes_off() {
        let pivot = [0.3, 1.0, 0.2];
        let rig = single(Motion::Flash, pivot);
        let reach = |activity: Activity, u: f32| {
            let pose = rig.pose(activity, u);
            // Where a point a tile from the pivot is carried: the flash's size.
            let p = apply(&pose[1], [pivot[0], pivot[1] + 1.0, pivot[2]]);
            assert!(
                close(apply(&pose[1], pivot), pivot),
                "the flash grows from its pivot"
            );
            p[1] - pivot[1]
        };
        assert!((reach(Activity::Attack, FIRE_AT + 0.04) - 1.0).abs() < 1e-3);
        for u in [0.0, 0.3, FIRE_AT - 0.01, FIRE_AT + 0.12, 0.9] {
            assert!(reach(Activity::Attack, u) < 0.01, "{u}");
        }
        for activity in [Activity::Idle, Activity::Move, Activity::Work] {
            for s in 0..POSE_SAMPLES {
                assert!(reach(activity, s as f32 / POSE_SAMPLES as f32) < 0.01);
            }
        }
        // At least one keyframe of the attack shows it at full size.
        let shown = (0..POSE_SAMPLES)
            .filter(|s| reach(Activity::Attack, *s as f32 / POSE_SAMPLES as f32) > 0.9)
            .count();
        assert!((1..=3).contains(&shown), "{shown}");
    }

    #[test]
    fn a_barrel_recoils_along_its_length_and_runs_out_again() {
        // A barrel lying along +z (its own y): it kicks back towards −z.
        let mut rig = Rig::default();
        rig.add(Joint {
            parent: 0,
            pivot: [0.0, 0.7, -0.4],
            frame: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
            scale: 1.0,
            motion: Motion::Recoil(0.14),
        })
        .unwrap();
        let at = |activity: Activity, u: f32| apply(&rig.pose(activity, u)[1], [0.0, 0.7, 0.5]);
        let kicked = at(Activity::Attack, FIRE_AT + 0.03);
        assert!(close(kicked, [0.0, 0.7, 0.36]), "{kicked:?}");
        for u in [0.0, 0.3, FIRE_AT, 0.95] {
            assert!(close(at(Activity::Attack, u), [0.0, 0.7, 0.5]), "{u}");
        }
        assert!(close(at(Activity::Move, FIRE_AT + 0.03), [0.0, 0.7, 0.5]));
    }

    #[test]
    fn a_sailing_hull_heels_only_under_way() {
        let rig = single(Motion::Float(deg(8.0)), [0.0; 3]);
        // The mean lean about the hull's length (x) over a cycle.
        let mean_heel = |activity: Activity| {
            (0..POSE_SAMPLES)
                .map(|s| {
                    let pose = rig.pose(activity, s as f32 / POSE_SAMPLES as f32);
                    // The hull's up axis tips towards +z by the heel.
                    pose[1][2][1].atan2(pose[1][1][1])
                })
                .sum::<f32>()
                / POSE_SAMPLES as f32
        };
        assert!((mean_heel(Activity::Move) - deg(8.0)).abs() < deg(0.5));
        assert!(mean_heel(Activity::Idle).abs() < deg(0.5));
        assert!(mean_heel(Activity::Attack).abs() < deg(0.5));
    }

    #[test]
    fn keyframes_hold_every_joint_of_every_sample() {
        let rig = figure(style());
        let texels = rig.keyframes();
        assert_eq!(texels.len(), 4 * POSE_SAMPLES * rig.joints.len() * 3);
        // Joint 0 is the model: always the identity.
        assert_eq!(texels[0], IDENTITY[0]);
        assert_eq!(rig.period(Activity::Move), DEFAULT_PERIODS[1]);
        let mut set = rig.clone();
        set.suggest_period(Activity::Move, 0.6);
        set.suggest_period(Activity::Move, 2.0);
        assert_eq!(set.period(Activity::Move), 0.6);
    }

    #[test]
    fn activities_are_named() {
        for activity in Activity::ALL {
            assert_eq!(Activity::from_name(activity.name()), Some(activity));
        }
        assert_eq!(Activity::from_name("dance"), None);
        assert!(track(0.5, 1.0, &[(0.5, 3.0)]) == 3.0);
    }
}
