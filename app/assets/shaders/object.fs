#version 330
// Baked feature objects up close: the part's program over its baked colour, snow above the
// snow line on upward faces, lit and shadowed like the ground. The objects fade in and out
// with the material level of detail by a screen-door pattern.
in vec3 fragNormal;
in vec3 fragWorld;
in vec3 fragColour;
flat in vec3 fragTint;
flat in int fragProgram;
out vec4 finalColor;
uniform float detail;
uniform float planetRadius;
uniform int hasSnow;
uniform float snowLine;
uniform vec3 snowColour;
/*LIGHTING*/
/*NOISE*/
/*MATERIALS*/
void main() {
    vec3 normal = normalize(fragNormal);
    if (!gl_FrontFacing) normal = -normal;
    vec3 up = planet == 1 ? normalize(fragWorld) : vec3(0.0, 1.0, 0.0);
    float px = length(fwidth(fragWorld));
    vec4 m = objectMaterial(fragProgram, fragWorld, fragColour, px);
    vec3 albedo = m.rgb;
    if (hasSnow == 1) {
        float altitude = planet == 1 ? length(fragWorld) - planetRadius : fragWorld.y;
        float wobble = valueNoise(fragWorld * 1.7) - 0.5;
        float cover = smoothstep(snowLine - 0.15, snowLine + 0.15, altitude + wobble * 0.9)
                    * smoothstep(0.2, 0.55, dot(normal, up));
        if (cover > 0.0) {
            vec4 snow = snowMaterial(fragWorld, snowColour, snowColour * vec3(0.84, 0.89, 0.97), 1.5, px);
            albedo = mix(albedo, snow.rgb, cover);
        }
    }
    vec3 bumped = patternBump(normal, m.a, fragWorld);
    finalColor = vec4(albedo * fragTint * terrainLight(fragWorld, bumped), 1.0);
    // Decided last, so the derivatives above see the whole pixel quad.
    if (hash13(vec3(floor(gl_FragCoord.xy), 3.0)) > detail) discard;
}
