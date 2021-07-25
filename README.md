# Sampan

Fix panorama pictures from Samsung phones. These pictures are not standard JPEG
and cannot be opened by most programs. They're also unnecessary large due to
the embedded video file. This program makes them valid JPEG and also strips
the video.

The program is also capable or removing the motion video from pictures taken
with Samsung phones and of removing similar videos from wide selfies.

Typically, it reduces the file size in about half, without any loss of picture
quality.

`Sampan` can also run in batch mode and fix all your images at once.

# How to install

On debian/ubuntu: Download & install deb from release page.

On other linuxes:

```bash
curl -o /usr/local/bin/sampan -L https://github.com/frapa/sampan/releases/latest/download/sampan
chmod +x /usr/local/bin/sampan
```

On Windows: Download exe and run. (Add to PATH if necessary)

# How to use

There are many options, but the basics are:

```bash
# Single file
sampan path/to/image.jpg -o output.jpg
# Dry run on folder
sampan path/to/*.jpg -d
# Replace files with stripped version (ensure you have a backup!)
sampan path/to/*.jpg -i
```
