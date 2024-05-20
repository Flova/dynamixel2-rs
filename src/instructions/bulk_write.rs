use super::{instruction_id, packet_id, BulkWriteData};
use crate::endian::{write_u16_le, write_u8_le};
use crate::{Bus, WriteError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously write arbitrary data ranges to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	/// Unlike the sync write instruction, a bulk write allows you to write a different amount of data to a different address for each motor.
	///
	/// The data for multi-byte registers should serialized as little-endian.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one write.
	///
	/// This function also panics if the data length for a motor exceeds the capacity of a `u16`.
	///
	/// # Example
	/// ```no_run
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// use dynamixel2::Bus;
	/// use dynamixel2::instructions::BulkWriteData;
	/// use std::time::Duration;
	///
	/// let mut bus = Bus::open("/dev/ttyUSB0", 57600, Duration::from_millis(20))?;
	/// bus.bulk_write(&[
	///   // Write a u32 value of 2000 to register 116 of motor 1.
	///   BulkWriteData {
	///     motor_id: 1,
	///     address: 116,
	///     data: 2000u32.to_le_bytes().as_slice(),
	///   },
	///   // Write a u16 value of 300 to register 102 of motor 2.
	///   BulkWriteData {
	///     motor_id: 2,
	///     address: 102,
	///     data: 300u16.to_le_bytes().as_slice(),
	///   },
	/// ])?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn bulk_write<'a, I, T>(&mut self, writes: &'a I) -> Result<(), WriteError>
	where
		&'a I: IntoIterator,
		<&'a I as IntoIterator>::IntoIter: Clone,
		<&'a I as IntoIterator>::Item: std::borrow::Borrow<BulkWriteData<T>>,
		T: AsRef<[u8]>,
	{
		use std::borrow::Borrow;

		let writes = writes.into_iter();
		let mut parameter_count = 0;
		for write in writes.clone() {
			let write = write.borrow();
			let data = write.data.as_ref();
			if data.len() > u16::MAX.into() {
				panic!(
					"bulk_write: data length ({}) for motor {} exceeds maximum size of {}",
					data.len(),
					write.motor_id,
					u16::MAX
				);
			}
			parameter_count += 5 + data.len();
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::BULK_WRITE, parameter_count, |buffer| {
			let mut offset = 0;
			for write in writes {
				let write = write.borrow();
				let data = write.data.as_ref();
				let buffer = &mut buffer[offset..];
				offset += 5 + data.len();
				write_u8_le(&mut buffer[0..], write.motor_id);
				write_u16_le(&mut buffer[1..], write.address);
				write_u16_le(&mut buffer[3..], data.len() as u16);
				buffer[5..][..data.len()].copy_from_slice(data);
			}
		})
	}
}
