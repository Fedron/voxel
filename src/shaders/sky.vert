#version 460

in vec3 position;

out float height;

uniform mat4 mvp;

void main() {
    height = position.y;

    gl_Position = mvp * vec4(position, 1.0);
    gl_Position.z = gl_Position.w;
}
