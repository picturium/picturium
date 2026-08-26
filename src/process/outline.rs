use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{BTreeSet, HashMap, HashSet};

const MAX_DEPTH: usize = 64;

/// Prune outline entries for removed pages / objects
pub(super) fn prune(document: &mut Document, removed: &BTreeSet<ObjectId>) {
    if removed.is_empty() {
        return;
    }

    let Some(outlines_id) = outlines_id(document) else {
        return;
    };

    let first = document
        .get_dictionary(outlines_id)
        .and_then(|outlines| outlines.get(b"First"))
        .and_then(Object::as_reference)
        .ok();

    let names = named_pages(document);
    let items = read_level(document, first, &names, &mut HashSet::new(), 0);
    let kept = filter(items, removed);

    let (Some(first), Some(last)) = (kept.first(), kept.last()) else {
        if let Ok(catalog) = document.catalog_mut() {
            catalog.remove(b"Outlines");
        }

        return;
    };

    let (first, last) = (first.id, last.id);
    let visible = write_level(document, outlines_id, &kept);

    if let Ok(outlines) = document.get_dictionary_mut(outlines_id) {
        outlines.set("First", Object::Reference(first));
        outlines.set("Last", Object::Reference(last));
        outlines.set("Count", visible);
    }
}

struct Item {
    id: ObjectId,
    open: bool,
    page: Option<ObjectId>,
    dest: Option<Object>,
    children: Vec<Item>,
}

struct Kept {
    id: ObjectId,
    open: bool,
    dest: Option<Object>,
    retarget: bool,
    children: Vec<Kept>,
}

fn outlines_id(document: &Document) -> Option<ObjectId> {
    document.catalog().ok()?.get(b"Outlines").ok()?.as_reference().ok()
}

fn read_level(document: &Document, first: Option<ObjectId>, names: &HashMap<Vec<u8>, ObjectId>, visited: &mut HashSet<ObjectId>, depth: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut current = first;

    while let Some(id) = current {
        if depth >= MAX_DEPTH || !visited.insert(id) {
            break;
        }

        let Ok(item) = document.get_dictionary(id) else {
            break;
        };

        let child = item.get(b"First").and_then(Object::as_reference).ok();
        let dest = destination(document, item).cloned();

        items.push(Item {
            id,
            open: item.get(b"Count").and_then(Object::as_i64).unwrap_or(0) >= 0,
            page: dest.as_ref().and_then(|dest| dest_page(document, dest, names, 0)),
            dest,
            children: read_level(document, child, names, visited, depth + 1),
        });

        current = item.get(b"Next").and_then(Object::as_reference).ok();
    }

    items
}

fn destination<'a>(document: &'a Document, item: &'a Dictionary) -> Option<&'a Object> {
    if let Ok(dest) = item.get(b"Dest") {
        return Some(dest);
    }

    let action = document.dereference(item.get(b"A").ok()?).ok()?.1.as_dict().ok()?;

    if action.get(b"S").and_then(Object::as_name).ok()? != b"GoTo" {
        return None;
    }

    action.get(b"D").ok()
}

fn dest_page(document: &Document, dest: &Object, names: &HashMap<Vec<u8>, ObjectId>, depth: usize) -> Option<ObjectId> {
    if depth >= MAX_DEPTH {
        return None;
    }

    match dest {
        Object::Reference(_) => dest_page(document, document.dereference(dest).ok()?.1, names, depth + 1),
        Object::Dictionary(dict) => dest_page(document, dict.get(b"D").ok()?, names, depth + 1),
        Object::Array(dest) => dest.first()?.as_reference().ok(),
        Object::String(name, _) | Object::Name(name) => names.get(name.as_slice()).copied(),
        _ => None,
    }
}

fn named_pages(document: &Document) -> HashMap<Vec<u8>, ObjectId> {
    let mut names = HashMap::new();

    let Ok(catalog) = document.catalog() else {
        return names;
    };

    if let Ok(dests) = catalog.get_deref(b"Dests", document).and_then(Object::as_dict) {
        for (name, dest) in dests.iter() {
            if let Some(page) = dest_page(document, dest, &HashMap::new(), 0) {
                names.insert(name.clone(), page);
            }
        }
    }

    if let Ok(tree) = catalog.get_deref(b"Names", document).and_then(Object::as_dict).and_then(|tree| tree.get_deref(b"Dests", document)).and_then(Object::as_dict) {
        read_name_tree(document, tree, &mut names, 0);
    }

    names
}

fn read_name_tree(document: &Document, node: &Dictionary, names: &mut HashMap<Vec<u8>, ObjectId>, depth: usize) {
    if depth >= MAX_DEPTH {
        return;
    }

    if let Ok(kids) = node.get(b"Kids").and_then(Object::as_array) {
        for kid in kids {
            if let Ok(kid) = kid.as_reference().and_then(|id| document.get_dictionary(id)) {
                read_name_tree(document, kid, names, depth + 1);
            }
        }
    }

    if let Ok(entries) = node.get(b"Names").and_then(Object::as_array) {
        for entry in entries.chunks(2) {
            let [Object::String(name, _), dest] = entry else {
                continue;
            };

            if let Some(page) = dest_page(document, dest, &HashMap::new(), 0) {
                names.insert(name.clone(), page);
            }
        }
    }
}

fn filter(items: Vec<Item>, removed: &BTreeSet<ObjectId>) -> Vec<Kept> {
    items
        .into_iter()
        .filter_map(|item| {
            let children = filter(item.children, removed);

            if item.page.is_none_or(|page| !removed.contains(&page)) {
                return Some(Kept {
                    id: item.id,
                    open: item.open,
                    dest: item.dest,
                    retarget: false,
                    children,
                });
            }

            if children.is_empty() {
                return None;
            }

            Some(Kept {
                id: item.id,
                open: item.open,
                dest: children.iter().find_map(|child| child.dest.clone()),
                retarget: true,
                children,
            })
        })
        .collect()
}

fn write_level(document: &mut Document, parent: ObjectId, items: &[Kept]) -> i64 {
    let mut visible = 0;

    for (index, item) in items.iter().enumerate() {
        let below = write_level(document, item.id, &item.children);

        let Ok(entry) = document.get_dictionary_mut(item.id) else {
            continue;
        };

        entry.set("Parent", Object::Reference(parent));

        match index.checked_sub(1).and_then(|previous| items.get(previous)) {
            Some(previous) => entry.set("Prev", Object::Reference(previous.id)),
            None => {
                entry.remove(b"Prev");
            }
        }

        match items.get(index + 1) {
            Some(next) => entry.set("Next", Object::Reference(next.id)),
            None => {
                entry.remove(b"Next");
            }
        }

        match (item.children.first(), item.children.last()) {
            (Some(first), Some(last)) => {
                entry.set("First", Object::Reference(first.id));
                entry.set("Last", Object::Reference(last.id));
                entry.set("Count", if item.open { below } else { -below });
            }
            _ => {
                entry.remove(b"First");
                entry.remove(b"Last");
                entry.remove(b"Count");
            }
        }

        if item.retarget {
            entry.remove(b"A");

            match &item.dest {
                Some(dest) => entry.set("Dest", dest.clone()),
                None => {
                    entry.remove(b"Dest");
                }
            }
        }

        visible += 1 + if item.open { below } else { 0 };
    }

    visible
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    /// A `pages`-page document with no outline yet.
    fn document(pages: usize) -> (Document, Vec<ObjectId>) {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();

        let page_ids: Vec<ObjectId> = (0..pages)
            .map(|_| {
                document.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                })
            })
            .collect();

        document.objects.insert(pages_id, Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => pages as i64,
            "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
        }));

        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog);

        (document, page_ids)
    }

    fn add_entry(document: &mut Document, title: &str, dest: Object) -> ObjectId {
        document.add_object(dictionary! {
            "Title" => Object::string_literal(title),
            "Dest" => dest,
        })
    }

    /// Chain `entries` as siblings and hang them off a new `/Outlines` root.
    fn add_outlines(document: &mut Document, entries: &[ObjectId]) -> ObjectId {
        let outlines_id = document.new_object_id();
        link(document, outlines_id, entries);

        document.objects.insert(outlines_id, Object::Dictionary(dictionary! {
            "Type" => "Outlines",
            "First" => Object::Reference(entries[0]),
            "Last" => Object::Reference(entries[entries.len() - 1]),
            "Count" => entries.len() as i64,
        }));

        document.catalog_mut().unwrap().set("Outlines", Object::Reference(outlines_id));
        outlines_id
    }

    fn link(document: &mut Document, parent: ObjectId, entries: &[ObjectId]) {
        for (index, id) in entries.iter().enumerate() {
            let entry = document.get_dictionary_mut(*id).unwrap();
            entry.set("Parent", Object::Reference(parent));

            if index > 0 {
                entry.set("Prev", Object::Reference(entries[index - 1]));
            }

            if let Some(next) = entries.get(index + 1) {
                entry.set("Next", Object::Reference(*next));
            }
        }
    }

    fn page_dest(page: ObjectId) -> Object {
        Object::Array(vec![Object::Reference(page), "Fit".into()])
    }

    /// Titles of the surviving entries, depth first, as the reader would walk them.
    fn titles(document: &Document) -> Vec<String> {
        fn walk(document: &Document, first: Option<ObjectId>, titles: &mut Vec<String>) {
            let mut current = first;

            while let Some(id) = current {
                let entry = document.get_dictionary(id).unwrap();
                let title = entry.get(b"Title").and_then(Object::as_str).unwrap();
                titles.push(String::from_utf8(title.to_vec()).unwrap());

                walk(document, entry.get(b"First").and_then(Object::as_reference).ok(), titles);
                current = entry.get(b"Next").and_then(Object::as_reference).ok();
            }
        }

        let mut collected = Vec::new();

        if let Some(outlines_id) = outlines_id(document) {
            let first = document
                .get_dictionary(outlines_id)
                .and_then(|outlines| outlines.get(b"First"))
                .and_then(Object::as_reference)
                .ok();

            walk(document, first, &mut collected);
        }

        collected
    }

    #[test]
    fn removes_entries_pointing_at_deleted_pages() {
        let (mut document, pages) = document(3);

        let entries: Vec<ObjectId> = pages
            .iter()
            .enumerate()
            .map(|(index, page)| add_entry(&mut document, &format!("page {}", index + 1), page_dest(*page)))
            .collect();

        let outlines_id = add_outlines(&mut document, &entries);
        prune(&mut document, &BTreeSet::from([pages[1]]));

        assert_eq!(titles(&document), vec!["page 1", "page 3"]);

        let outlines = document.get_dictionary(outlines_id).unwrap();
        assert_eq!(outlines.get(b"Count").unwrap().as_i64().unwrap(), 2);
        assert_eq!(outlines.get(b"Last").unwrap().as_reference().unwrap(), entries[2]);

        let survivor = document.get_dictionary(entries[2]).unwrap();
        assert_eq!(survivor.get(b"Prev").unwrap().as_reference().unwrap(), entries[0]);
        assert!(!survivor.has(b"Next"));
        assert!(!document.get_dictionary(entries[0]).unwrap().has(b"Prev"));
    }

    #[test]
    fn keeps_a_parent_whose_children_survive_and_retargets_it() {
        let (mut document, pages) = document(3);

        let child = add_entry(&mut document, "child", page_dest(pages[2]));
        let parent = add_entry(&mut document, "parent", page_dest(pages[1]));

        link(&mut document, parent, &[child]);
        let parent_dict = document.get_dictionary_mut(parent).unwrap();
        parent_dict.set("First", Object::Reference(child));
        parent_dict.set("Last", Object::Reference(child));
        parent_dict.set("Count", 1);

        let outlines_id = add_outlines(&mut document, &[parent]);
        prune(&mut document, &BTreeSet::from([pages[1]]));

        assert_eq!(titles(&document), vec!["parent", "child"]);

        let parent_dict = document.get_dictionary(parent).unwrap();
        assert_eq!(dest_page(&document, parent_dict.get(b"Dest").unwrap(), &HashMap::new(), 0), Some(pages[2]));

        // The parent is open, so it and its child are both visible.
        assert_eq!(document.get_dictionary(outlines_id).unwrap().get(b"Count").unwrap().as_i64().unwrap(), 2);
    }

    #[test]
    fn drops_a_whole_branch_when_nothing_under_it_survives() {
        let (mut document, pages) = document(3);

        let child = add_entry(&mut document, "child", page_dest(pages[1]));
        let parent = add_entry(&mut document, "parent", page_dest(pages[1]));
        let survivor = add_entry(&mut document, "survivor", page_dest(pages[0]));

        link(&mut document, parent, &[child]);
        let parent_dict = document.get_dictionary_mut(parent).unwrap();
        parent_dict.set("First", Object::Reference(child));
        parent_dict.set("Last", Object::Reference(child));
        parent_dict.set("Count", 1);

        add_outlines(&mut document, &[parent, survivor]);
        prune(&mut document, &BTreeSet::from([pages[1]]));

        assert_eq!(titles(&document), vec!["survivor"]);
    }

    #[test]
    fn resolves_named_destinations() {
        let (mut document, pages) = document(2);

        let dest = document.add_object(Object::Array(vec![Object::Reference(pages[1]), "Fit".into()]));
        let names = document.add_object(dictionary! {
            "Names" => vec![Object::string_literal("section"), Object::Reference(dest)],
        });
        let catalog = document.catalog_mut().unwrap();
        catalog.set("Names", dictionary! { "Dests" => Object::Reference(names) });

        let kept = add_entry(&mut document, "page 1", page_dest(pages[0]));
        let named = add_entry(&mut document, "named", Object::string_literal("section"));
        add_outlines(&mut document, &[kept, named]);

        prune(&mut document, &BTreeSet::from([pages[1]]));
        assert_eq!(titles(&document), vec!["page 1"]);
    }

    #[test]
    fn removes_the_outline_when_no_entry_survives() {
        let (mut document, pages) = document(2);

        let entry = add_entry(&mut document, "page 2", page_dest(pages[1]));
        add_outlines(&mut document, &[entry]);

        prune(&mut document, &BTreeSet::from([pages[1]]));

        assert!(!document.catalog().unwrap().has(b"Outlines"));
        assert!(titles(&document).is_empty());
    }
}
