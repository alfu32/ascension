// The material library: one function per program. Each returns the albedo and a relief
// height in tiles at world point `p`. `a` and `b` are the material's two colours, `s` its
// pattern size in tiles and `px` the width of a pixel in tiles there.
// Program ids: 1 grass, 2 sand, 3 stone, 4 snow, 5 forest floor, 7 foliage, 8 bark, 9 crop;
// on composed models also 10 plaster, 11 brick, 12 timber, 13 roof tile, 14 thatch, 15 metal,
// 16 cloth (0 is a plain colour).
uniform float time;

// Grass: patches of two greens, tufts of blades, and gusts of wind sweeping across, turning
// the blades to show their lighter sides.
vec4 grassMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    vec3 q = p / s;
    float patches = fbm(q * 0.6, 2, px / s * 0.6);
    vec3 colour = mix(a, b, smoothstep(0.3, 0.8, patches) * 0.75);
    float tuft = 7.0 / s;
    float fine = keep(tuft, px);
    float blades = fine > 0.0 ? valueNoise(p * tuft) : 0.5;
    float wind = sin(dot(p, vec3(0.57, 0.23, 0.79)) * 0.85 - time * 1.7 + patches * 5.0);
    float gust = smoothstep(0.35, 1.0, wind) * (0.55 + 0.45 * sin(dot(p, vec3(0.13, 0.37, 0.21)) + time * 0.4));
    colour *= 1.0 + fine * (blades - 0.5) * 0.55;
    colour = mix(colour, colour * 1.22 + vec3(0.035, 0.045, 0.0), gust * 0.6);
    return vec4(colour, fine * (blades - 0.5) * 0.035 + (patches - 0.5) * 0.04);
}

// Sand: dunes of two tones, wind ripples and grain.
vec4 sandMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    vec3 q = p / s;
    float dunes = fbm(q * 0.4, 3, px / s * 0.4);
    float warp = valueNoise(q * 0.7) * 4.0;
    float frequency = 5.0 / s;
    float ripples = sin(dot(p, vec3(0.83, 0.12, 0.55)) * frequency * 6.2832 / 6.0 + warp);
    float wave = keep(frequency / 6.0, px);
    float grain = keep(38.0 / s, px) > 0.0 ? valueNoise(p * 38.0 / s) : 0.5;
    vec3 colour = mix(a, b, clamp(dunes * 0.9 + ripples * 0.08 * wave, 0.0, 1.0));
    colour *= 1.0 + (grain - 0.5) * 0.2 * keep(38.0 / s, px);
    return vec4(colour, ripples * 0.012 * wave + (dunes - 0.5) * 0.05);
}

// Rough stone: two tones of rock, coarse grain and darker hollows.
vec4 stoneMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    vec3 q = p / s;
    float body = fbm(q, 3, px / s);
    float fine = keep(6.0 / s, px);
    float grain = fine > 0.0 ? valueNoise(q * 6.0) : 0.5;
    vec3 colour = mix(b, a, smoothstep(0.22, 0.78, body));
    colour *= 1.0 + (grain - 0.5) * 0.3 * fine;
    return vec4(colour, (body - 0.5) * 0.2 * s + (grain - 0.5) * 0.025 * fine);
}

// Snow and ice: soft drifts, blue in the hollows, a little sparkle.
vec4 snowMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    float drift = fbm(p / s * 0.7, 3, px / s * 0.7);
    vec3 colour = mix(b, a, smoothstep(0.3, 0.7, drift));
    float sparkle = step(0.992, hash13(floor(p * 45.0))) * keep(45.0, px);
    colour += vec3(sparkle * 0.3);
    return vec4(colour, (drift - 0.5) * 0.08 * s);
}

// Forest floor: moss and leaf litter.
vec4 forestFloorMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    float moss = fbm(p / s * 0.8, 3, px / s * 0.8);
    float litter = keep(12.0 / s, px) > 0.0 ? valueNoise(p * 12.0 / s) : 0.5;
    vec3 colour = mix(a, b, smoothstep(0.4, 0.72, moss));
    colour *= 0.85 + (litter - 0.5) * 0.4 * keep(12.0 / s, px) + 0.15;
    return vec4(colour, (moss - 0.5) * 0.06);
}

// Leaves: clusters with dark gaps between them.
vec4 foliageMaterial(vec3 p, vec3 base, float s, float px) {
    float clusters = valueNoise(p / s);
    float leaves = valueNoise(p * 4.0 / s);
    float gaps = smoothstep(0.6, 0.78, valueNoise(p / s * 0.7 + 11.0));
    vec3 colour = base * (0.78 + 0.45 * clusters);
    colour *= 1.0 + (leaves - 0.5) * 0.5 * keep(4.0 / s, px);
    colour *= 1.0 - gaps * 0.45;
    return vec4(colour, clusters * 0.05 + leaves * 0.02);
}

// Bark: fine vertical fissures.
vec4 barkMaterial(vec3 p, vec3 base, float px) {
    float fissure = valueNoise(p * vec3(30.0, 4.0, 30.0));
    return vec4(base * (0.75 + 0.4 * fissure * keep(30.0, px) + 0.2 * (1.0 - keep(30.0, px))), fissure * 0.01);
}

// Crop blades: veins along the blade, darker towards the base.
vec4 cropMaterial(vec3 p, vec3 base, float s, float px) {
    float vein = valueNoise(p * 26.0);
    return vec4(base * (0.88 + 0.24 * vein * keep(26.0, px)), 0.0);
}

// A land material by program id.
vec4 landMaterial(int program, vec3 p, vec3 a, vec3 b, float s, float px) {
    if (program == 1) return grassMaterial(p, a, b, s, px);
    if (program == 2) return sandMaterial(p, a, b, s, px);
    if (program == 3) return stoneMaterial(p, a, b, s, px);
    if (program == 4) return snowMaterial(p, a, b, s, px);
    if (program == 5) return forestFloorMaterial(p, a, b, s, px);
    return vec4(a, 0.0);
}

// An object part by program id; `base` is the part's baked colour.
vec4 objectMaterial(int program, vec3 p, vec3 base, float px) {
    if (program == 7) return foliageMaterial(p, base, 0.22, px);
    if (program == 8) return barkMaterial(p, base, px);
    if (program == 9) return cropMaterial(p, base, 0.3, px);
    if (program == 3) return stoneMaterial(p, base, base * 0.62, 0.9, px);
    if (program == 4) return snowMaterial(p, base, base * 0.85, 1.5, px);
    return vec4(base, 0.0);
}

// ── Composed models ─────────────────────────────────────────────────────────────────────────
// Model programs work in the model's own frame (`p`, tiles) on a face with normal `n`: `uv`
// runs across the face (u horizontal, v up the face or up the roof slope).
vec2 surfaceUV(vec3 p, vec3 n) {
    if (abs(n.y) > 0.97) return p.xz;
    vec3 t = normalize(vec3(-n.z, 0.0, n.x));
    return vec2(dot(p, t), dot(p, normalize(cross(t, n))));
}

// Plaster or mud render: soft blotches, faint streaks running down from the top, fine grain.
vec4 plasterMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float blotch = fbm(p / s * 0.9, 3, px / s * 0.9);
    vec3 colour = mix(b, a, smoothstep(0.2, 0.8, blotch));
    float streaks = keep(6.0 / s, px);
    float streak = streaks > 0.0 ? valueNoise(vec3(uv.x * 6.0 / s, uv.y * 0.5 / s, 3.0)) : 0.5;
    colour *= 1.0 - 0.08 * smoothstep(0.55, 0.9, streak) * streaks;
    float fine = keep(28.0, px);
    float grain = fine > 0.0 ? valueNoise(p * 28.0) : 0.5;
    colour *= 1.0 + (grain - 0.5) * 0.1 * fine;
    return vec4(colour, (blotch - 0.5) * 0.012 + (grain - 0.5) * 0.002 * fine);
}

// Brick or ashlar: courses 0.45 `s` high in running bond, blocks `s` long, mortar in `b`.
vec4 brickMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float course = s * 0.45;
    float row = floor(uv.y / course);
    float x = uv.x / s + row * 0.5;
    float id = hash13(vec3(floor(x), row, 7.0));
    float dx = min(fract(x), 1.0 - fract(x)) * s;
    float dy = min(fract(uv.y / course), 1.0 - fract(uv.y / course)) * course;
    float joint = 0.012 + 0.02 * s;
    float shown = keep(1.0 / course, px);
    float mortar = (1.0 - smoothstep(joint * 0.5, joint * 0.5 + px, min(dx, dy))) * shown;
    vec3 block = a * (0.84 + 0.3 * id);
    block *= 0.94 + 0.12 * fbm(p / s * 2.0, 2, px / s * 2.0);
    vec3 colour = mix(block, b, mortar);
    return vec4(colour, (1.0 - mortar) * 0.012);
}

// Planks or logs `s` wide running up the face, with grain and dark seams.
vec4 timberMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float plank = floor(uv.x / s);
    float f = fract(uv.x / s);
    float id = hash13(vec3(plank, 3.0, 11.0));
    float shown = keep(1.0 / s, px);
    float fine = keep(14.0 / s, px);
    float grain = fine > 0.0 ? valueNoise(vec3(uv.x * 14.0 / s, uv.y * 1.2 / s, plank)) : 0.5;
    vec3 colour = mix(b, a, 0.35 + 0.65 * grain) * (0.82 + 0.3 * id);
    float bulge = sin(f * 3.14159);
    float seam = (1.0 - smoothstep(0.0, 0.09 + px / s, min(f, 1.0 - f))) * shown;
    colour *= mix(1.0, 0.45, seam) * (0.88 + 0.12 * bulge * shown);
    return vec4(colour, bulge * 0.012 * shown + grain * 0.002 * fine);
}

// Barrel roof tiles: columns `s` wide in rows 0.8 `s` deep, each row shading the next.
vec4 roofTileMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float depth = s * 0.8;
    float row = floor(uv.y / depth);
    float fv = fract(uv.y / depth);
    float u = uv.x / s + row * 0.5;
    float id = hash13(vec3(floor(u), row, 5.0));
    float shown = keep(1.0 / depth, px);
    float barrel = 0.5 + 0.5 * cos(fract(u) * 6.28318);
    vec3 colour = mix(b, a, 0.35 + 0.65 * mix(0.6, barrel, shown)) * (0.88 + 0.22 * id);
    float lip = mix(1.0, 0.62 + 0.38 * smoothstep(0.0, 0.3, fv), shown);
    colour *= lip;
    colour *= 0.92 + 0.12 * fbm(p * 0.7, 2, px * 0.7);
    return vec4(colour, (barrel * 0.012 + fv * 0.018) * shown);
}

// Thatch: straw strands running down the slope in courses 1.6 `s` deep with slightly ragged
// lower edges, darker under each course and weathered grey in patches.
vec4 thatchMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float fine = keep(24.0 / s, px);
    float strands = fine > 0.0 ? valueNoise(vec3(uv.x * 24.0 / s, uv.y * 1.2 / s, 1.0)) : 0.5;
    float tufts = keep(5.0 / s, px);
    float tuft = tufts > 0.0 ? valueNoise(vec3(uv.x * 5.0 / s, uv.y * 0.6 / s, 6.0)) : 0.5;
    float depth = s * 1.6;
    float ragged = (valueNoise(vec3(uv.x * 9.0 / s, 2.0, 0.0)) - 0.5) * 0.12 * tufts;
    float fv = fract(uv.y / depth + ragged);
    float shown = keep(1.0 / depth, px);
    vec3 colour = mix(b, a, 0.35 + 0.45 * mix(0.5, strands, fine) + 0.2 * (tuft - 0.5));
    colour *= mix(1.0, 0.74 + 0.26 * smoothstep(0.0, 0.35, fv), shown);
    float weather = fbm(p / s * 0.3, 3, px / s * 0.3);
    colour = mix(colour, colour * vec3(0.78, 0.77, 0.74), smoothstep(0.5, 0.8, weather) * 0.7);
    return vec4(colour, (strands * 0.008 * fine + fv * 0.02 * shown));
}

// Metal: brushed along the face, a little brighter where it faces up.
vec4 metalMaterial(vec3 p, vec2 uv, vec3 n, vec3 a, vec3 b, float s, float px) {
    float fine = keep(40.0, px);
    float brush = fine > 0.0 ? valueNoise(vec3(uv.x * 2.0, uv.y * 40.0, 9.0)) : 0.5;
    vec3 colour = mix(b, a, 0.55 + 0.45 * brush);
    colour *= 0.95 + 0.25 * max(n.y, 0.0);
    return vec4(colour, brush * 0.002);
}

// Cloth: soft folds across the face and a fine weave.
vec4 clothMaterial(vec3 p, vec2 uv, vec3 a, vec3 b, float s, float px) {
    float fold = 0.5 + 0.5 * sin(uv.x / s * 1.9 + valueNoise(vec3(uv * 0.8 / s, 4.0)) * 3.0);
    float weave = keep(60.0, px) * (0.5 + 0.25 * (sin(uv.x * 380.0) + sin(uv.y * 380.0)));
    vec3 colour = mix(b, a, 0.55 + 0.45 * fold);
    colour *= 0.94 + 0.1 * weave;
    return vec4(colour, fold * 0.01);
}

// A part of a composed model by program id, `a` and `b` its two colours, `s` its pattern size.
vec4 modelMaterial(int program, vec3 p, vec3 n, vec3 a, vec3 b, float s, float px) {
    vec2 uv = surfaceUV(p, n);
    if (program == 10) return plasterMaterial(p, uv, a, b, s, px);
    if (program == 11) return brickMaterial(p, uv, a, b, s, px);
    if (program == 12) return timberMaterial(p, uv, a, b, s, px);
    if (program == 13) return roofTileMaterial(p, uv, a, b, s, px);
    if (program == 14) return thatchMaterial(p, uv, a, b, s, px);
    if (program == 15) return metalMaterial(p, uv, n, a, b, s, px);
    if (program == 16) return clothMaterial(p, uv, a, b, s, px);
    if (program == 3) return stoneMaterial(p, a, b, s, px);
    if (program == 0) return vec4(a, 0.0);
    return objectMaterial(program, p, a, px);
}
