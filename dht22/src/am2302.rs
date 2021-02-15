use std::cmp::PartialEq;
use std::ops::BitXor;
use std::ops::Shl;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadingError {
    #[error("Wrong number of input bites, should be 40")]
    WrongBitsCount,

    #[error("Something wrong with conversion to bytes")]
    MalformedData(#[from] ConversionError),

    #[error("Parity Bit Validation Failed")]
    ParityBitMismatch,

    #[error("Value is outside of specification")]
    OutOfSpecValue,
}

#[derive(Debug, PartialEq)]
pub struct Reading {
    temperature: f32,
    humidity: f32,
}

impl Reading {
    pub fn from_binary_slice(data: &[u8]) -> Result<Self, ReadingError> {
        if data.len() != 40 {
            return Err(ReadingError::WrongBitsCount);
        }

        let bytes: Vec<u8> = data
            .chunks(8)
            .map(|chunk| convert(chunk))
            .collect::<Result<Vec<_>, _>>()?;

        if bytes.len() < 5 {
            return Err(ReadingError::WrongBitsCount);
        }

        let check_sum: u8 = bytes[..4]
            .iter()
            .fold(0 as u8, |result, &value| result.overflowing_add(value).0);
        if check_sum != bytes[4] {
            return Err(ReadingError::ParityBitMismatch);
        }

        let raw_humidity: u16 = (bytes[0] as u16) * 256 + bytes[1] as u16;
        let raw_temperature: i16 = if bytes[2] >= 128 {
            bytes[3] as i16 * -1
        } else {
            (bytes[2] as i16) * 256 + bytes[3] as i16
        };

        let humidity: f32 = raw_humidity as f32 / 10.0;
        let temperature: f32 = raw_temperature as f32 / 10.0;

        if temperature > 81.0 || temperature < -41.0 {
            return Err(ReadingError::OutOfSpecValue);
        }
        if humidity < 0.0 || humidity > 99.9 {
            return Err(ReadingError::OutOfSpecValue);
        }

        Ok(Reading {
            temperature,
            humidity,
        })
    }

    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    pub fn humidity(&self) -> f32 {
        self.humidity
    }
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Overflow")]
    Overflow,

    #[error("NonBinaryInput")]
    NonBinaryInput,
}

pub fn convert<T: PartialEq + From<u8> + BitXor<Output = T> + Shl<Output = T> + Clone>(
    bits: &[u8],
) -> Result<T, ConversionError> {
    let l = std::mem::size_of::<T>();
    if bits.len() > (l * 8) {
        return Err(ConversionError::Overflow);
    }
    if bits.iter().filter(|&&bit| bit != 0 && bit != 1).count() > 0 {
        return Err(ConversionError::NonBinaryInput);
    }

    Ok(bits.iter().fold(T::from(0), |result, &bit| {
        (result << T::from(1)) ^ T::from(bit)
    }))
}
