<p align="center">
  <h3 align="center">HeightMapTo3dTerrain</h3>

  <p align="center">
    This is command line tool made with .NET to generate 3d terrain from grayscale HeightMaps
	<br>
    <a href="https://github.com/shanto462/HeightMapTo3dTerrain"><strong>Explore the docs »</strong></a>
    <a href="https://github.com/shanto462/HeightMapTo3dTerrain/issues">Report Bug</a>
    <a href="https://github.com/shanto462/HeightMapTo3dTerrain/issues">Request Feature</a>
  </p>
</p>



<!-- TABLE OF CONTENTS -->
<details open="open">
  <summary><h2 style="display: inline-block">Table of Contents</h2></summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgements">Acknowledgements</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

This tool generates a wavefront obj file from grayscale heightmaps.


### Built With

* .NET

<!-- GETTING STARTED -->
## Getting Started

To get a local copy up and running by following these simple steps.

### Prerequisites

This Visual Studio Project targets multiple runtimes.
You will need **.NET 5.0, .NET Core 3.1** and .**NET Framework**.

### Installation

1. Clone the repo
   ```sh
   git clone https://github.com/shanto462/HeightMapTo3dTerrain.git
   ```
2. Restore Nugets packages and Build

Or
1. Download from Release.

<!-- USAGE EXAMPLES -->
## Usage

HeightMapTo3dTerrain.exe -sourcefile -destinationfile -minHeight -maxHeight

**Make sure input image has same width and height**

HeightMapTo3dTerrain.exe /dir/a.png /dir/out.obj -200 200

<!-- ROADMAP -->
## Roadmap

See the [open issues](https://github.com/shanto462/HeightMapTo3dTerrain/issues) for a list of proposed features (and known issues).



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to be learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.



<!-- CONTACT -->
## Contact

Your Name - Shanto
Email - <shalahuddinshanto@gmail.com>

Project Link: [https://github.com/github_username/repo_name](https://github.com/shanto462/HeightMapTo3dTerrain)
