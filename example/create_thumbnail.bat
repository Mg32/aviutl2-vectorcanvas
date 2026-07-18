ffmpeg -y -i example.webp -c:v libwebp -lossless 0 -compression_level 6 -qscale 70 -loop 0 -vf "crop=1080:1080:(in_w-1080)/2:(in_h-1080)/2,scale=412:412" thumbnail.webp
