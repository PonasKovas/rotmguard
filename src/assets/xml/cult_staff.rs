use anyhow::{bail, Context, Result};
use xmltree::{Element, XMLNode};

// Original is 345, so 360-345 = 15
const ARC_GAP: &str = "15";

// If this object is the cult staff, changes it's ArcGap
// Returns true if its the cult staff
pub fn cult_staff(object: &mut Element) -> Result<bool> {
	if let Some("Staff of Unholy Sacrifice") = object.attributes.get("id").map(|s| s.as_str()) {
		for param in &mut object.children {
			let param = match param {
				XMLNode::Element(p) => p,
				_ => continue,
			};
			if param.name != "ArcGap" {
				continue;
			}
			let gap = match param
				.children
				.get_mut(0)
				.context("cult staff ArcGap no children")?
			{
				XMLNode::Text(gap) => gap,
				_ => bail!("cult staff ArcGap not text"),
			};

			gap.clear();
			*gap += ARC_GAP;

			return Ok(true);
		}
	}

	Ok(false)
}
