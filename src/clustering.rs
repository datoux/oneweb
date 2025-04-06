use std::fmt;

pub struct Pixel {
    pub x: u8,
    pub y: u8,
    pub value: u16,
    pub neighbor_mask: u8,
    pub neighbors: [i8; 8],
}

#[allow(dead_code)]
impl Pixel {
    pub fn new(x: u8, y: u8, value: u16) -> Pixel {
        Pixel {
            x,
            y,
            value,
            neighbor_mask: 0,
            neighbors: [-1; 8],
        }
    }

    pub fn add_neighbor(&mut self, dir: usize, pix_idx: i8) {
        self.neighbors[dir] = pix_idx;
        self.neighbor_mask |= 1 << dir;
    }
}

impl fmt::Debug for Pixel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Pixel {{ x: {}, y: {}, val: {}}}",
            self.x, self.y, self.value
        )
    }
}

#[derive(Debug, Default)]
pub struct Cluster {
    pub pixels: Vec<Pixel>,
}

#[allow(dead_code)]
impl Cluster {
    pub fn new() -> Cluster {
        Cluster { pixels: Vec::new() }
    }

    pub fn add_pixel(&mut self, pixel: Pixel) {
        self.pixels.push(pixel);
    }
}

#[allow(dead_code)]
pub struct Clusterer {
    pub vec: Vec<Cluster>,
}

#[allow(dead_code)]
impl Clusterer {
    pub fn new() -> Clusterer {
        Clusterer { vec: Vec::new() }
    }

    pub fn search_frame(&self, frame: &[u16], width: i64, height: i64) -> Vec<Cluster> {
        let mut clusters: Vec<Cluster> = Vec::new();

        const DIRX: [i8; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
        const DIRY: [i8; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
        const UNTESTED: i64 = -1;
        let mut mask: Vec<i64> = vec![UNTESTED; frame.len()];

        for (idx, value) in frame.iter().enumerate() {
            if value == &0 || mask[idx] != UNTESTED {
                continue;
            }

            let x: u8 = (idx % 256) as u8;
            let y: u8 = (idx / 256) as u8;

            let mut cluster = Cluster::new();
            let first_pixel = Pixel::new(x, y, *value);
            cluster.add_pixel(first_pixel);
            mask[idx] = 0;

            // go through all pixels in the cluster and surroundings (pixels added as they are found)
            let mut pix_idx = 0;
            while pix_idx < cluster.pixels.len() {
                let x = cluster.pixels[pix_idx].x as i64;
                let y = cluster.pixels[pix_idx].y as i64;

                // find all neighbours 8-way search
                for dir in 0..8 {
                    let dx = x + DIRX[dir] as i64;
                    let dy = y + DIRY[dir] as i64;
                    if dx < 0 || dy < 0 || dx >= width || dy >= height {
                        continue;
                    }

                    let didx: usize = (dy * width + dx) as usize;
                    if frame[didx] == 0 {
                        continue;
                    }

                    if mask[didx] == UNTESTED {
                        // new pixel, not part of any cluster
                        let pixel = Pixel::new(dx as u8, dy as u8, frame[didx]);
                        cluster.add_pixel(pixel);
                        mask[didx] = (pix_idx + 1) as i64;
                    } else {
                        // pixel already part of a cluster
                    }

                    let idx = mask[didx];
                    cluster.pixels[pix_idx].add_neighbor(dir, idx as i8);
                }

                pix_idx += 1;
            }

            clusters.push(cluster);
        }

        clusters
    }
}
