#version 430 core

out vec4 color;

void main()
{
    float b = 16;
    float c = (int(gl_FragCoord.x / b) + int(gl_FragCoord.y / b)) % 2;
    color = vec4(1.0f * c, 0.0f, 1.0f * c, 1.0f);
}