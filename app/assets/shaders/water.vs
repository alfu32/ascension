#version 330
// The sea: a sphere at sea level on a planet, the plane y = 0 on a flat map.
in vec3 vertexPosition;
in vec4 vertexColor;
uniform mat4 mvp;
out vec4 fragColor;
out vec3 fragPosition;
void main() {
    fragColor = vertexColor;
    fragPosition = vertexPosition;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
