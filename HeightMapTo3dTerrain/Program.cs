﻿/*
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


using HeightMapTo3dTerrain;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

if (args.Length < 4)
{
    ShowHelp();
    return;
}

if (!int.TryParse(args[2], out int minHeight) || !int.TryParse(args[3], out int maxHeight))
{
    ShowHelp();
    return;
}

if (new ObjectFileGenerator(args[0], args[1], minHeight, maxHeight).Generate())
{
    Console.WriteLine("Successfully Generated!");
}
else
{
    Console.WriteLine("Terrain Generation Failed!");
}

static void ShowHelp()
{
    Console.WriteLine
    (
        @"Usage:
                HeightMapTo3dTerrain.exe -sourceFile -destinationFile -minHeight -maxHeight
                Make sure input image has same width and height
                HeightMapTo3dTerrain.exe /dir/a.png /dir/out.obj -200 200"
    );
}