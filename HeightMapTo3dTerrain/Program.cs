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
using System.Threading.Tasks;

namespace HeightMapTo3dTerrain
{
    class Program
    {
        public static void ShowHelp()
        {
            Console.WriteLine
            (
                "Usage \nHeightMapTo3dTerrain.exe -sourcefile -destinationfile \nHeightMapTo3dTerrain.exe /dir/a.png /dir/out.png"
            );
        }

        static void Main(string[] args)
        {
            if (args.Length < 4)
            {
                ShowHelp();
                return;
            }

            int minHeight = 0;
            int maxHeight = 0;

            if (!int.TryParse(args[2], out minHeight) || !int.TryParse(args[3], out maxHeight))
            {
                ShowHelp();
                return;
            }

            if (new _3dGenerator(args[0], args[1], minHeight, maxHeight).Generate())
            {
                Console.WriteLine("Successfully Generated!");
            }
            else
            {
                Console.WriteLine("Terrain Generation Failed!");
            }

        }
    }
}
