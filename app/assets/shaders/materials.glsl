// The material library: one function per program. Each returns the albedo and a relief
// height in tiles at world point `p`. `a` and `b` are the material's two colours, `s` its
// pattern size in tiles and `px` the width of a pixel in tiles there.
// Program ids: 1 grass, 2 sand, 3 stone, 4 snow, 5 forest floor, 7 foliage, 8 bark, 9 crop.
uniform float time;

// Grass: patches of two greens, tufts of blades, and gusts of wind sweeping across, turning
// the blades to show their lighter sides.
vec4 grassMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    vec3 q = p / s;
    float patches = fbm(q * 0.6, 3, px / s * 0.6);
    vec3 colour = mix(a, b, smoothstep(0.3, 0.8, patches) * 0.75);
    float tuft = 7.0 / s;
    float fine = keep(tuft, px);
    float blades = valueNoise(p * tuft) * 0.55 + valueNoise(p * tuft * 2.7 + 5.0) * 0.45;
    float wind = sin(dot(p, vec3(0.57, 0.23, 0.79)) * 0.85 - time * 1.7 + patches * 5.0);
    float gust = smoothstep(0.35, 1.0, wind) * (0.55 + 0.45 * valueNoise(p * 0.3 + time * 0.1));
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
    float grain = valueNoise(p * 38.0 / s);
    vec3 colour = mix(a, b, clamp(dunes * 0.9 + ripples * 0.08 * wave, 0.0, 1.0));
    colour *= 1.0 + (grain - 0.5) * 0.2 * keep(38.0 / s, px);
    return vec4(colour, ripples * 0.012 * wave + (dunes - 0.5) * 0.05);
}

// Rough stone: two tones of rock, coarse grain and darker hollows.
vec4 stoneMaterial(vec3 p, vec3 a, vec3 b, float s, float px) {
    vec3 q = p / s;
    float body = fbm(q, 3, px / s);
    float grain = valueNoise(q * 6.0);
    float fine = keep(6.0 / s, px);
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
    float litter = valueNoise(p * 12.0 / s);
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
