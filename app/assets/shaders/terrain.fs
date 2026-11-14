#version 330
// Land: map colours far out, the material programs up close (the host of the material
// library, after meshcraft's procedural.fs). `detail` follows the camera distance; each
// fragment also loses detail with its own distance from the eye.
in vec3 fragNormal;
in vec4 fragColor;
in vec3 fragWorld;
in vec3 fragShade;
in vec4 fragWeightsA;
in vec3 fragWeightsB;
out vec4 finalColor;
uniform vec3 viewPosition;
uniform float detail;
uniform vec2 detailRange;
uniform int slotProgram[7];
uniform vec3 slotColour[7];
uniform vec3 slotAlt[7];
uniform float slotScale[7];
/*LIGHTING*/
/*NOISE*/
/*MATERIALS*/
// Adds slot `i`'s material at weight `w` (the slot index is a constant at each call, so no
// local array is indexed at run time).
void addSlot(int i, float w, float px, inout vec3 sum, inout float height) {
    if (w > 0.004) {
        vec4 m = landMaterial(slotProgram[i], fragWorld, slotColour[i], slotAlt[i], slotScale[i], px);
        sum += m.rgb * w;
        height += m.a * w;
    }
}

void main() {
    vec3 normal = normalize(fragNormal);
    float px = length(fwidth(fragWorld));
    float near = detail * (1.0 - smoothstep(detailRange.x, detailRange.y, distance(viewPosition, fragWorld)));
    float total = dot(fragWeightsA, vec4(1.0)) + dot(fragWeightsB, vec3(1.0));
    vec3 albedo = fragColor.rgb;
    float height = 0.0;
    if (near > 0.002 && total > 0.002) {
        vec3 sum = vec3(0.0);
        addSlot(0, fragWeightsA.x, px, sum, height);
        addSlot(1, fragWeightsA.y, px, sum, height);
        addSlot(2, fragWeightsA.z, px, sum, height);
        addSlot(3, fragWeightsA.w, px, sum, height);
        addSlot(4, fragWeightsB.x, px, sum, height);
        addSlot(5, fragWeightsB.y, px, sum, height);
        addSlot(6, fragWeightsB.z, px, sum, height);
        float amount = near * min(total, 1.0);
        albedo = mix(fragColor.rgb, sum / total * fragShade, amount);
        height *= amount / total;
    }
    vec3 lit = normal;
    if (detail > 0.002) lit = patternBump(normal, height, fragWorld);
    finalColor = vec4(albedo * terrainLight(fragWorld, lit), 1.0);
}
