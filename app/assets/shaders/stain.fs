#version 330
// Stains on the ground (task 80): soot darkest at the centre, ash flecks and churned earth
// towards a ragged rim that varies with each stain's seed, lit by the day and night and the
// shadows like the ground (the soot's light comes from its vertices, the corners of the tiles
// under it, so a fragment costs one noise fetch: soot can cover much of the screen). Stains
// blend over the ground; overlapping ones darken it towards the soot colour, never past it.
// Debris is lit as solid pieces.
in vec3 fragWorld;
in vec3 fragNormal;
in vec4 fragColour;
in vec2 fragLocal;
in float fragFade;
in vec3 fragLight;
flat in float fragSeed;
flat in float fragDebris;
out vec4 finalColor;
/*LIGHTING*/
/*NOISE*/
const vec3 SOOT = vec3(0.085, 0.076, 0.068);
const vec3 EARTH = vec3(0.21, 0.16, 0.11);
const vec3 ASH = vec3(0.40, 0.39, 0.37);
const float DEEPEST = 0.86;
void main() {
    vec3 n = normalize(fragNormal);
    if (fragDebris > 0.5) {
        // Weathered towards the earth as it settles, fading with the soot (owner review: debris
        // must not outlast the mark it lies in).
        vec3 weathered = mix(fragColour.rgb, EARTH * 1.5, (1.0 - fragFade) * 0.8);
        finalColor = vec4(weathered * terrainLight(fragWorld, n), fragFade);
        return;
    }
    // Outside the farthest rim nothing is drawn; most of a stain's tiles end here, cheaply.
    float r = length(fragLocal);
    if (r > 1.3) discard;
    // A ragged rim: smooth functions of the direction from the centre, varied by the seed.
    vec2 dir = fragLocal / max(r, 1e-4);
    float k = fragSeed * 6.2832;
    float rim = 0.86 + 0.08 * sin(3.1 * dir.x + 2.3 * dir.y + k * 7.0)
        + 0.06 * sin(5.3 * dir.y - 3.7 * dir.x + k * 13.0);
    vec3 s = vec3(fragSeed * 61.0, fragSeed * 23.0, fragSeed * 7.0);
    float blotch = valueNoise(fragWorld * 1.3 + s);
    float t = r / rim + (blotch - 0.5) * 0.3;
    if (t >= 1.0) discard;
    float px = length(fwidth(fragWorld));
    // Ash flecks: a few cells of a fine grid, each a soft round speck.
    vec3 cell = fragWorld * 5.0;
    vec3 inner = fract(cell) - 0.5;
    float fleck = step(0.8, hash13(floor(cell) + s)) * (1.0 - smoothstep(0.15, 0.4, length(inner)))
        * keep(5.0, px);
    float core = 1.0 - smoothstep(0.35, 1.0, t);
    float alpha = min(core * (0.75 + 0.5 * (blotch - 0.5)), DEEPEST);
    vec3 colour = mix(SOOT, EARTH, smoothstep(0.3, 0.9, t + (blotch - 0.5) * 0.3));
    colour = mix(colour, ASH, fleck * 0.6 * (1.0 - t));
    finalColor = vec4(colour * fragLight, alpha * fragFade);
}
