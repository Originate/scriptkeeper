use yaml_rust;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Marker {
    pub line: usize,
    pub col: usize,
}

impl Marker {
    pub fn line(&self) -> usize {
        let Marker { line, .. } = self;
        *line
    }
}

impl<'a> From<&'a yaml_rust::Marker> for Marker {
    fn from(marker: &'a yaml_rust::Marker) -> Self {
        Marker {
            line: marker.line(),
            col: marker.col(),
        }
    }
}

impl From<yaml_rust::Marker> for Marker {
    fn from(marker: yaml_rust::Marker) -> Self {
        Marker {
            line: marker.line(),
            col: marker.col(),
        }
    }
}
