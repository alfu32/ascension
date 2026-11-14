#version 330
// Composed models: the part's program over its two colours in the model's own frame (so the
// pattern sits still on a turning unit), owner colour on team faces, the baked contact shade,
// lit and shadowed like the terrain objects. A model under construction shows only what lies
// below `reveal` (model units).
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
/*LIGHTING*/
/*NOISE*/
/*MATERIALS*/
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
    vec3 bumped = patternBump(normal, m.a, fragWorld);
    finalColor = vec4(m.rgb * fragShade * terrainLight(fragWorld, bumped), 1.0);
}
