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



using SkiaSharp;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Numerics;
using System.Text;

namespace HeightMapTo3dTerrain
{
    public class HeightMap : IDisposable
    {
        private readonly SKBitmap heightMap;

        private bool disposedValue;

        public HeightMap(string fileLocation)
        {
            if (!File.Exists(fileLocation))
                throw new FileNotFoundException($"Couldn't found the file {fileLocation}");

            heightMap = SKBitmap.Decode(File.ReadAllBytes(fileLocation));

            if (heightMap.Width != heightMap.Height)
                throw new NotSupportedException("Please provide heightmap with same width and height!");
        }


        private SKColor GetIndex(int i, int j)
        {
            return heightMap.GetPixel(i, j);
        }

        public Vector3 GetVector3(int i, int j)
        {
            var color = GetIndex(i, j);
            return new Vector3(color.Red / 255.0f, color.Green / 255.0f, color.Blue / 255.0f);
        }

        public int Height => heightMap.Height;

        public int Width => heightMap.Width;

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    heightMap.Dispose();
                }

                disposedValue = true;
            }
        }


        public void Dispose()
        {
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
