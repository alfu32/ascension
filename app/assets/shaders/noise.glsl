// Noise and relief helpers shared by the material programs (after meshcraft's
// patterns.glsl). Patterns are sampled in world space, in tiles.
float hash13(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

// Smooth value noise in 0..1 from one fetch of the noise texture: its red channel holds a
// random value per texel, its green channel the red value 37 and 17 texels on, so two z
// slices come from the same bilinear lookup.
uniform sampler2D noiseTexture;
float valueNoise(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    vec2 uv = i.xy + vec2(37.0, 17.0) * i.z + f.xy;
    vec2 rg = textureLod(noiseTexture, (uv + 0.5) / 256.0, 0.0).rg;
    return mix(rg.x, rg.y, f.z);
}

// How much of a pattern of `frequency` cycles per tile survives at a pixel `footprint` tiles
// wide: 1 while a cycle spans several pixels, 0 once it would alias.
float keep(float frequency, float footprint) {
    return clamp(1.6 - frequency * footprint * 2.0, 0.0, 1.0);
}

// Fractal noise in 0..1; octaves finer than a pixel fade to their mean.
float fbm(vec3 p, int octaves, float footprint) {
    float sum = 0.0;
    float amplitude = 0.5;
    float total = 0.0;
    for (int i = 0; i < 6; ++i) {
        float kept = keep(1.0, footprint);
        if (i >= octaves || kept <= 0.0) break;
        sum += amplitude * kept * (valueNoise(p) - 0.5);
        total += amplitude;
        p = p * 2.03 + vec3(1.7, 9.2, 3.1);
        amplitude *= 0.5;
        footprint *= 2.03;
    }
    return 0.5 + sum / max(total, 1e-4);
}

// Derivative bump mapping: tilts `n` by the screen-space slope of the height `h` (tiles).
vec3 patternBump(vec3 n, float h, vec3 world) {
    vec3 dpdx = dFdx(world);
    vec3 dpdy = dFdy(world);
    vec3 r1 = cross(dpdy, n);
    vec3 r2 = cross(n, dpdx);
    float det = dot(dpdx, r1);
    vec3 g = dFdx(h) * r1 + dFdy(h) * r2;
    if (abs(det) < 1e-10) return n;
    return normalize(abs(det) * n - sign(det) * g);
}
