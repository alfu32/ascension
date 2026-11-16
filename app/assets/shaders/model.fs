#version 330
// Composed models: the part's program over its two colours in the model's own frame (so the
// pattern sits still on a turning unit), owner colour on team faces, the baked contact shade,
// lit and shadowed like the terrain objects. A model under construction shows only what lies
// below `reveal` (model units). In a star system (`starLight` = 1) the star at the origin of
// the frame lights it from its own direction, and the ball `starShade` (centre, radius; radius 0
// for none) hides the star from what lies in its shadow; `starFill` lights faces that look up
// off the system's plane, so a ship seen from above reads against black space.
in vec3 fragNormal;
in vec3 fragWorld;
in vec3 fragLocal;
in vec3 fragLocalNormal;
in vec3 fragColour;
in vec3 fragAlt;
in float fragShade;
flat in float fragScale;
flat in vec3 fragTeam;
flat in int fragProgram;
flat in int fragTeamFace;
out vec4 finalColor;
uniform float reveal;
uniform int starLight;
uniform vec4 starShade;
uniform float starFill;
/*LIGHTING*/
/*NOISE*/
/*MATERIALS*/
vec3 starLighting(vec3 world, vec3 normal) {
    vec3 toStar = normalize(-world);
    float fill = starFill * max(normal.y, 0.0);
    vec3 day = vec3(DAY_AMBIENT + DIRECT * max(dot(normal, toStar), 0.0) + fill);
    float lit = 1.0;
    if (starShade.w > 0.0) {
        vec3 toBall = starShade.xyz - world;
        float along = dot(toBall, toStar);
        float off = length(toBall - toStar * along);
        lit = along > 0.0 ? smoothstep(starShade.w * (1.0 - TERMINATOR), starShade.w * (1.0 + TERMINATOR), off) : 1.0;
    }
    return mix(NIGHT_AMBIENT, day, lit);
}
void main() {
    if (fragLocal.y > reveal) discard;
    vec3 normal = normalize(fragNormal);
    if (!gl_FrontFacing) normal = -normal;
    float px = length(fwidth(fragLocal));
    vec3 a = fragColour;
    vec3 b = fragAlt;
    if (fragTeamFace == 1) {
        // The owner's colour, keeping a little of the material so cloth still reads as cloth.
        a = mix(fragTeam, fragColour, 0.12);
        b = mix(fragTeam * 0.62, fragAlt, 0.12);
    }
    vec4 m = modelMaterial(fragProgram, fragLocal, normalize(fragLocalNormal), a, b, fragScale, px);
    if (fragProgram == 18) {
        finalColor = vec4(m.rgb, 1.0);
        return;
    }
    vec3 bumped = patternBump(normal, m.a, fragWorld);
    vec3 light = starLight == 1 ? starLighting(fragWorld, bumped) : terrainLight(fragWorld, bumped);
    finalColor = vec4(m.rgb * fragShade * light, 1.0);
}
