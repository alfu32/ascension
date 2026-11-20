#version 330
// Grass blades (task 83): Meshcraft's parametric grass program `meshcraft.grass.v1` (its
// procedural.vs, generation "blades") on the ground triangles around the eye. One domain entry
// is one land triangle and the first of its candidates; instance i draws candidate
// first + i % bladesPerTriangle of entry i / bladesPerTriangle (the attribute divisor). A blade
// is three segments of two triangles. Changed for the game: the triangles are already in planet
// space (no model matrix); the density follows the eye distance (Meshcraft's LOD per
// triangle, times the far fade) and the grass share of the ground; blades shrink as they fade;
// the base colour is the grass material of the ground at the root, with its map shade.
layout(location=0) in vec3 domainA;
layout(location=1) in vec3 domainB;
layout(location=2) in vec3 domainC;
// Seed (low and high 16 bits), first candidate, grass material slot.
layout(location=3) in vec4 domainSeed;
// Grass share of the ground blend at each corner.
layout(location=4) in vec3 grassShare;
// Map shade (fog of war, borders) at each corner, packed r*65536 + g*256 + b over 0..2.
layout(location=5) in vec3 mapShade;
// Smooth ground normals at the corners.
layout(location=6) in vec3 normalA;
layout(location=7) in vec3 normalB;
layout(location=8) in vec3 normalC;
uniform mat4 viewProjection;
uniform int bladesPerTriangle;
uniform float density;
uniform vec3 viewPosition;
// Meshcraft's LOD distance, the end of full height, the end of the fade and the start of the
// coarse blades (tiles from the eye).
uniform vec4 reach;
// Width of a pixel per tile of distance, for the material's pattern filter.
uniform float pixelAngle;
uniform vec3 slotColour[7];
uniform vec3 slotAlt[7];
uniform float slotScale[7];
uniform float u_height;
uniform float u_width;
uniform float u_curvature;
uniform float u_wind;
uniform float u_variation;
uniform int u_seed;
// Brightness gain that keeps a meadow's mean tone at the plain ground's under the shading below.
uniform float toneGain;
out vec3 worldPosition;
out vec3 groundNormal;
out vec4 color;
// 1 for a full blade, falling to 0 where the blades fade out.
out float bladeFade;
/*NOISE*/
/*MATERIALS*/
uint hash(uint x) { x ^= x >> 16u; x *= 0x7feb352du; x ^= x >> 15u; x *= 0x846ca68bu; return x ^ (x >> 16u); }
float random(uint x) { return float(hash(x) & 0x00ffffffu) / 16777216.0; }
// Past reach.w the blades are fewer by the square of this and as many times wider.
float widen(float d) {
    return max(d / reach.w, 1.0);
}
float lod(float d) {
    float r = d / reach.x;
    float w = widen(d);
    return 1.0 / (1.0 + r * r) / (w * w) * (1.0 - smoothstep(reach.y, reach.z, d));
}
vec3 unpackShade(float code) {
    return vec3(floor(code / 65536.0), mod(floor(code / 256.0), 256.0), mod(code, 256.0)) / 255.0 * 2.0;
}
void discardBlade() {
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
    worldPosition = vec3(0.0);
    groundNormal = vec3(0.0, 1.0, 0.0);
    color = vec4(0.0);
    bladeFade = 0.0;
}
void main() {
    vec3 normal = normalize(cross(domainB - domainA, domainC - domainA));
    if (dot(normal, domainA) < 0.0) normal = -normal;
    uint candidate = uint(domainSeed.z) + uint(gl_InstanceID % bladesPerTriangle);
    uint key = hash(uint(domainSeed.x) | (uint(domainSeed.y) << 16u)) ^ hash(candidate) ^ uint(u_seed);
    float area = length(cross(domainB - domainA, domainC - domainA)) * 0.5;
    float most = max(grassShare.x, max(grassShare.y, grassShare.z));
    float near = distance(viewPosition, (domainA + domainB + domainC) / 3.0);
    // Fractional last blade uses a stable sample, so density varies smoothly.
    if (float(candidate) + random(key + 3u) >= area * density * lod(near) * most) {
        discardBlade();
        return;
    }
    float r = sqrt(random(key + 1u)), s = random(key + 2u);
    vec3 weights = vec3(1.0 - r, r * (1.0 - s), r * s);
    // Fewer blades where the ground blends towards another material.
    if (random(key + 7u) * most >= dot(weights, grassShare)) {
        discardBlade();
        return;
    }
    vec3 root = domainA * weights.x + domainB * weights.y + domainC * weights.z;
    float away = distance(viewPosition, root);
    float fade = 1.0 - smoothstep(reach.y, reach.z, away);
    if (fade <= 0.0) {
        discardBlade();
        return;
    }
    vec3 tangent = normalize(domainB - domainA), bitangent = normalize(cross(normal, tangent));
    float angle = random(key + 4u) * 6.28318530718;
    vec3 side = tangent * cos(angle) + bitangent * sin(angle), bend = cross(normal, side);
    // Meshcraft's three segments of two triangles, drawn from the blade's eight corners (two
    // per row, four rows) through an index list, so shared corners are shaded once.
    vec2 uv = vec2(float(gl_VertexID % 2) * 2.0 - 1.0, float(gl_VertexID / 2) / 3.0);
    float h = u_height * (0.7 + 0.6 * random(key + 5u)) * fade;
    float wind = u_wind * sin(time * 1.7 + root.x * 0.5 + root.z * 0.7 + angle);
    vec3 position = root + normal * (uv.y * h)
        + side * (uv.x * u_width * widen(away) * 0.5 * (1.0 - uv.y))
        + bend * (uv.y * uv.y * h * (u_curvature + wind));
    // Base shading "material colour": the grass of the ground at the root.
    int slot = int(domainSeed.w);
    float px = away * pixelAngle;
    vec3 ground = landMaterial(1, root, slotColour[slot], slotAlt[slot], slotScale[slot], px).rgb;
    vec3 shade = unpackShade(mapShade.x) * weights.x + unpackShade(mapShade.y) * weights.y
        + unpackShade(mapShade.z) * weights.z;
    color = vec4(ground * shade, 1.0);
    // Meshcraft's seeded variation and darker base; fading blades take the ground's own tone.
    float tone = (1.0 - u_variation * random(key + 6u)) * (0.65 + 0.35 * uv.y) * toneGain;
    color.rgb *= mix(1.0, tone, fade);
    bladeFade = fade;
    groundNormal = normalize(normalA * weights.x + normalB * weights.y + normalC * weights.z);
    worldPosition = position;
    gl_Position = viewProjection * vec4(position, 1.0);
}
