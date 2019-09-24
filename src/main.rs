extern crate crossbeam;
extern crate image;

use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;

use num::Complex;

//Traits
use std::io::Write;
use std::str::FromStr;

//<***Through the function escape-time()************>//
//We will determine how long it takes for the complex number c to leave the
//Mandelbrot set and become an infinitely number (well, actually we have 
//restricted this with norm_sqr() ).
//If it takes a really long time then we are dealing with a value 
//likely to be part of the mandelbrot set 
//and also if the limit is passed without it flying away, it is within the set
//
fn escape_time(c : Complex<f64>,limit : u32) -> Option<u32> {
    let mut z = Complex { re : 0.0 , im : 0.0 };
    for i in 0..limit {
        z = z * z + c;

        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}
//Parsing string values that are separated by a given character ('x' or comma)
//to yield two string values that are parsed to another type
fn parse_pair<T : FromStr>(s : &str, separator :char) -> Option<(T , T)>{
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]),T::from_str(&s[index+1..])) {
                (Ok(l) , Ok(r)) => Some(( l , r )),
                _ => None
            }
        }
    }
}


//Using parse_pair() function above to parse a string to a Complex number type.
fn parse_complex(s : &str) -> Option<Complex<f64>> {
    match parse_pair(s,',') {
        Some((re,im)) => Some(Complex { re , im }),
        None => None
    }
}


//<***********Converting Pixels to points on the Complex plane*******>//
fn pixel_to_point(
   bounds : (usize, usize ),
   pixel : ( usize , usize ),
   upper_left : Complex<f64>,
   lower_right : Complex<f64>
    ) -> Complex<f64>
{
    let width = lower_right.re - upper_left.re;
    let height = upper_left.im - lower_right.im;
    
    Complex {
        re : upper_left.re + pixel.0  as f64 / bounds.0 as f64 * width,
        im : upper_left.im - pixel.1 as f64 / bounds.1 as f64 * height
    }
}

//<********render function********************>//
//<*****Assigns grayscale pixel values to our window*********>//
fn render(
    pixels : &mut[u8],
    bounds : (usize,usize),
    upper_left : Complex<f64>,
    lower_right : Complex<f64>)
{
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            //We will move across the width of the image window
            //calling the pixel_to_point() function on all the individual points
            //moving across each row before moving to the next row
            let point = pixel_to_point(
                bounds,(column,row),upper_left,lower_right
                );
            //Since we have a mutable reference to the pixels slice variable
            //Lets change the pixel values for each point accordingly
            //We're working with single-number grayscale pixel values that 
            //represent the brightness of the pixel
            //The most common pixel format is the byte image, 
            //where this number is stored as an 8-bit integer 
            //giving a range of possible values from 0 to 255. 
            //Typically zero is black & 255 is white. 
            //Values in between make up the different shades of gray.
            //We use 255 as the limit of possible iterations it took
            //for us to find out whether we're dealing with a mandelbrot set
            pixels[column + bounds.0 * row] = match escape_time(point,255) {
                None => 0,
                Some(count) => 255 - count as u8
            }
        }
    }
}

fn write_image(filename : &str,pixels : &[u8], bounds : (usize , usize))
    -> Result<(), std::io::Error>
{
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels,
                   bounds.0 as u32,
                   bounds.1 as u32,
                   ColorType::Gray(8))?;
    Ok(())
}



//<<******************MAIN FUNCTION*****************>>//
//<<******************MAIN FUNCTION*****************>>//
fn main() {
    let available_cpus = num_cpus::get();
    //Returns the number of available CPUs of the current system
    
    let num_of_cores = num_cpus::get_physical();
    //Returns the number of physical cores of the current system.
    //
    println!( "Number of cpus = {} and number of physical cores = {}",
              available_cpus , num_of_cores);

    //println!("Hello, world!");
    let args : Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        writeln!(std::io::stderr(),
        "Usage : mandelbrot File Pixels Upperleft Lowerright")
            .unwrap();
        writeln!(std::io::stderr(),
        "Example : {} mandelbrot.png 1000x750 -1.20,0.34 -1.0,2.0", args[0])
            .unwrap();
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2],'x')
        .expect("Error parsing image dimensions");
    let upper_left = parse_complex(&args[3])
        .expect("ERROR parsing upper left complex corner point");
    let lower_right = parse_complex(&args[4])
        .expect("ERROR parsing lower right complex corner point.");
    
    //The statement below equates all the pixel values
    //in the image widow to zero
    let mut pixels = vec![0;bounds.0 * bounds.1];
    
    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;
    {
        //Height of a single band is rows_per_band
        //height of overall window/image is bounds.1 
        
        //Not sure why we add 1
        //we then need to obtain mutable non-overlapping iterable chunks 
        //of ChunkMut type
        //which we will then iterate over by transferring owneship of the 
        //elements to a closure using the into_iter() method.
        //
        //Using the enumerate method we can get the current iteration count (i)
        //as well as the value (band) returned by the next iteration.
        let bands : Vec<&mut[u8]> =
            pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i,band) in bands.into_iter().enumerate() {
                //top is essentially the pixel value at the top upper_left
                //corner of the band
                //for example, for the top-most band, if bounds.1 = 1000
                //and threads = 8, then 1000/8 = 250, so top = 250*0=0
                //and for the second band from the top=>250*1=250, and so on...
                let top = rows_per_band * i;
                
                //Since the bands value is one long vector slice value,
                //consisting of all the values of the band 
                //while being dimension-agnostic
                //dividing by the width (bounds.0) recovers our dimensions
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0 , height);
                
                let band_upper_left = 
                    pixel_to_point(bounds, (0, top),
                    upper_left,lower_right);

                let band_lower_right = 
                    pixel_to_point(bounds, (bounds.0,top + height),
                    upper_left,lower_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        });

    }

    write_image(&args[1], &pixels, bounds)
        .expect("error writing PNG file!!");


}
