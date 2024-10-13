/*
Copyright (c) 2021 Shalahuddin, Email: shalahuddinshanto@gmail.com

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/


using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Text;

namespace HeightMapTo3dTerrain;

public class ObjectFileGenerator(string inputFileLocation, string outputFileLocation, int heightMin, int heightMax, bool OpenAfterCompletion = true)
{
    public bool Generate()
    {
        try
        {
            if (File.Exists(outputFileLocation))
                File.Delete(outputFileLocation);

            using var file = new FileStream(outputFileLocation, FileMode.OpenOrCreate);

            using var writer = new StreamWriter(file);

            using var heightMap = new HeightMap(inputFileLocation);

            float min = float.MaxValue, max = float.MinValue;
            int Width = heightMap.Width, Height = heightMap.Height;

            var heightData = new float[heightMap.Width * heightMap.Height];

            for (int x = 0; x < Width; x++)
            {
                for (int y = 0; y < Height; y++)
                {
                    heightData[y + x * Width] = heightMap.GetVector3(x, y).Y;

                    if (heightData[y + x * Width] < min)
                        min = heightData[y + x * Width];
                    if (heightData[y + x * Width] > max)
                        max = heightData[y + x * Width];
                }
            }

            for (int x = 0; x < Width; x++)
            {
                for (int y = 0; y < Height; y++)
                {
                    heightData[y + x * Width] = (heightData[y + x * Width]) / ((max - min) + 1);
                }
            }

            var center = new Vector3(Width, (heightMax - heightMin) * .5f, Height) * .5f;


            List<Vector3> NormalList = [];
            List<Vector3> VertexList = [];

            List<string> normals = [];

            for (int x = 0; x < Width; x++)
            {
                for (int y = 0; y < Height; y++)
                {
                    float h = Lerp(heightMin, heightMax, heightData[y + x * Width]);
                    heightData[y + x * Width] = h;

                    var coord = new Vector3(x, h, y) - center;
                    VertexList.Add(coord);

                    writer.WriteLine($"v {coord.X} {coord.Y} {coord.Z}");

                    NormalList.Add(Vector3.Zero);
                }
            }

            int[] index = new int[(Width - 1) * (Height - 1) * 6];

            for (int x = 0; x < Width - 1; x++)
            {
                for (int y = 0; y < Height - 1; y++)
                {
                    index[(x + y * (Width - 1)) * 6] = ((x + 1) + (y + 1) * Height);
                    index[(x + y * (Width - 1)) * 6 + 1] = ((x + 1) + y * Height);
                    index[(x + y * (Width - 1)) * 6 + 2] = (x + y * Height);

                    index[(x + y * (Width - 1)) * 6 + 3] = ((x + 1) + (y + 1) * Height);
                    index[(x + y * (Width - 1)) * 6 + 4] = (x + y * Height);
                    index[(x + y * (Width - 1)) * 6 + 5] = (x + (y + 1) * Height);
                }
            }


            for (int i = 0; i < NormalList.Count; i++)
                NormalList[i] = new Vector3(0, 0, 0);

            for (int i = 0; i < index.Length / 3; i++)
            {
                int index1 = index[i * 3];
                int index2 = index[i * 3 + 1];
                int index3 = index[i * 3 + 2];

                Vector3 side1 = VertexList[index1] - VertexList[index3];
                Vector3 side2 = VertexList[index1] - VertexList[index2];
                Vector3 normal = Vector3.Cross(side1, side2);

                NormalList[index1] += normal;
                NormalList[index2] += normal;
                NormalList[index3] += normal;
            }

            for (int i = 0; i < NormalList.Count; i++)
            {
                NormalList[i] = Vector3.Normalize(NormalList[i]);
                normals.Add($"vn {NormalList[i].X} {NormalList[i].Y} {NormalList[i].Z}");
            }

            foreach (var normal in normals.Distinct())
            {
                writer.WriteLine(normal);
            }

            for (int x = 2; x < Width - 2; x++)
            {
                for (int y = 2; y < Height - 2; y++)
                {
                    var top = x - 1;
                    var left = y - 1;
                    var right = y + 1;
                    var bottom = x + 1;

                    if (top < Width && left < Height && top >= 0 && left >= 0)
                    {
                        writer.WriteLine($"f {Get1DIndex(top, y, Width)} {Get1DIndex(x, y, Width)} {Get1DIndex(x, left, Width)}");
                    }
                    if (top < Width && right < Height && top >= 0 && right >= 0)
                    {
                        writer.WriteLine($"f {Get1DIndex(top, y, Width)} {Get1DIndex(x, y, Width)} {Get1DIndex(x, right, Width)}");
                    }
                    if (left < Height && bottom < Width && left >= 0 && bottom >= 0)
                    {
                        writer.WriteLine($"f {Get1DIndex(bottom, y, Width)} {Get1DIndex(x, y, Width)} {Get1DIndex(x, left, Width)}");

                    }
                    if (right < Height && bottom < Width && right >= 0 && bottom >= 0)
                    {
                        writer.WriteLine($"f {Get1DIndex(bottom, y, Width)} {Get1DIndex(x, y, Width)} {Get1DIndex(x, right, Width)}");
                    }
                }
            }

            if (OpenAfterCompletion)
            {
                new Process
                {
                    StartInfo = new ProcessStartInfo(outputFileLocation)
                    {
                        UseShellExecute = true
                    }
                }.Start();
            }

            return true;
        }
        catch (Exception ex)
        {
            Console.WriteLine(ex.Message);
            return false;
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static int Get1DIndex(int i, int j, int Width)
    {
        return j + i * Width;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static float Lerp(float value1, float value2, float amount)
    {
        return value1 + (value2 - value1) * amount;
    }
}
