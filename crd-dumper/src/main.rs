use std::path::{Path, PathBuf};

use kube::{CustomResourceExt, Resource};
use kubizone_crds::{Record, Zone};
use kubizone_zonefile_crds::ZoneFile;

fn dump_crd<C>(path: &Path) -> Result<(), std::io::Error>
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    let directory = path.join(C::api_version(&()).as_ref());

    std::fs::create_dir_all(&directory)?;

    let name = C::kind(&());

    let path = directory.join(format!("{name}.yaml"));

    std::fs::write(
        path,
        format!("---\n{}", serde_yaml::to_string(&C::crd()).unwrap()),
    )
    .unwrap();

    Ok(())
}

fn main() {
    let Some(path) = std::env::args().skip(1).next() else {
        panic!("no output path provided");
    };

    let path = PathBuf::from(path);

    dump_crd::<Zone>(&path).unwrap();
    dump_crd::<Record>(&path).unwrap();
    dump_crd::<ZoneFile>(&path).unwrap();
}
