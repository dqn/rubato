use std::collections::HashMap;
use std::sync::OnceLock;

use super::string_property::StringProperty;
use crate::stubs::MainState;

/// Returns a StringProperty for the given ID.
pub fn get_string_property_by_id(id: i32) -> Option<Box<dyn StringProperty>> {
    let map = get_id_map();
    if map.contains_key(&id) {
        return Some(Box::new(DelegateStringProperty { id }));
    }
    None
}

/// Returns a StringProperty for the given name.
pub fn get_string_property_by_name(name: &str) -> Option<Box<dyn StringProperty>> {
    for st in STRING_TYPES.iter() {
        if st.name == name {
            return Some(Box::new(DelegateStringProperty { id: st.id }));
        }
    }
    None
}

fn get_id_map() -> &'static HashMap<i32, &'static str> {
    static ID_MAP: OnceLock<HashMap<i32, &'static str>> = OnceLock::new();
    ID_MAP.get_or_init(|| {
        let mut map = HashMap::new();
        for st in STRING_TYPES.iter() {
            map.insert(st.id, st.name);
        }
        map
    })
}

struct StringTypeEntry {
    id: i32,
    name: &'static str,
}

static STRING_TYPES: &[StringTypeEntry] = &[
    StringTypeEntry {
        id: 1,
        name: "rival",
    },
    StringTypeEntry {
        id: 2,
        name: "player",
    },
    StringTypeEntry {
        id: 3,
        name: "target",
    },
    StringTypeEntry {
        id: 10,
        name: "title",
    },
    StringTypeEntry {
        id: 11,
        name: "subtitle",
    },
    StringTypeEntry {
        id: 12,
        name: "fulltitle",
    },
    StringTypeEntry {
        id: 13,
        name: "genre",
    },
    StringTypeEntry {
        id: 14,
        name: "artist",
    },
    StringTypeEntry {
        id: 15,
        name: "subartist",
    },
    StringTypeEntry {
        id: 16,
        name: "fullartist",
    },
    StringTypeEntry {
        id: 40,
        name: "key1",
    },
    StringTypeEntry {
        id: 41,
        name: "key2",
    },
    StringTypeEntry {
        id: 42,
        name: "key3",
    },
    StringTypeEntry {
        id: 43,
        name: "key4",
    },
    StringTypeEntry {
        id: 44,
        name: "key5",
    },
    StringTypeEntry {
        id: 45,
        name: "key6",
    },
    StringTypeEntry {
        id: 46,
        name: "key7",
    },
    StringTypeEntry {
        id: 47,
        name: "key8",
    },
    StringTypeEntry {
        id: 48,
        name: "key9",
    },
    StringTypeEntry {
        id: 49,
        name: "key10",
    },
    StringTypeEntry {
        id: 61,
        name: "sort",
    },
    StringTypeEntry {
        id: 86,
        name: "chartreplication",
    },
    StringTypeEntry {
        id: 240,
        name: "key11",
    },
    StringTypeEntry {
        id: 241,
        name: "key12",
    },
    StringTypeEntry {
        id: 242,
        name: "key13",
    },
    StringTypeEntry {
        id: 243,
        name: "key14",
    },
    StringTypeEntry {
        id: 244,
        name: "key15",
    },
    StringTypeEntry {
        id: 245,
        name: "key16",
    },
    StringTypeEntry {
        id: 246,
        name: "key17",
    },
    StringTypeEntry {
        id: 247,
        name: "key18",
    },
    StringTypeEntry {
        id: 248,
        name: "key19",
    },
    StringTypeEntry {
        id: 249,
        name: "key20",
    },
    StringTypeEntry {
        id: 250,
        name: "key21",
    },
    StringTypeEntry {
        id: 251,
        name: "key22",
    },
    StringTypeEntry {
        id: 252,
        name: "key23",
    },
    StringTypeEntry {
        id: 253,
        name: "key24",
    },
    StringTypeEntry {
        id: 254,
        name: "key25",
    },
    StringTypeEntry {
        id: 255,
        name: "key26",
    },
    StringTypeEntry {
        id: 256,
        name: "key27",
    },
    StringTypeEntry {
        id: 257,
        name: "key28",
    },
    StringTypeEntry {
        id: 258,
        name: "key29",
    },
    StringTypeEntry {
        id: 259,
        name: "key30",
    },
    StringTypeEntry {
        id: 260,
        name: "key31",
    },
    StringTypeEntry {
        id: 261,
        name: "key32",
    },
    StringTypeEntry {
        id: 262,
        name: "key33",
    },
    StringTypeEntry {
        id: 263,
        name: "key34",
    },
    StringTypeEntry {
        id: 264,
        name: "key35",
    },
    StringTypeEntry {
        id: 265,
        name: "key36",
    },
    StringTypeEntry {
        id: 266,
        name: "key37",
    },
    StringTypeEntry {
        id: 267,
        name: "key38",
    },
    StringTypeEntry {
        id: 268,
        name: "key39",
    },
    StringTypeEntry {
        id: 269,
        name: "key40",
    },
    StringTypeEntry {
        id: 270,
        name: "key41",
    },
    StringTypeEntry {
        id: 271,
        name: "key42",
    },
    StringTypeEntry {
        id: 272,
        name: "key43",
    },
    StringTypeEntry {
        id: 273,
        name: "key44",
    },
    StringTypeEntry {
        id: 274,
        name: "key45",
    },
    StringTypeEntry {
        id: 275,
        name: "key46",
    },
    StringTypeEntry {
        id: 276,
        name: "key47",
    },
    StringTypeEntry {
        id: 277,
        name: "key48",
    },
    StringTypeEntry {
        id: 278,
        name: "key49",
    },
    StringTypeEntry {
        id: 279,
        name: "key50",
    },
    StringTypeEntry {
        id: 280,
        name: "key51",
    },
    StringTypeEntry {
        id: 281,
        name: "key52",
    },
    StringTypeEntry {
        id: 282,
        name: "key53",
    },
    StringTypeEntry {
        id: 283,
        name: "key54",
    },
    StringTypeEntry {
        id: 50,
        name: "skinname",
    },
    StringTypeEntry {
        id: 51,
        name: "skinauthor",
    },
    StringTypeEntry {
        id: 100,
        name: "skincategory1",
    },
    StringTypeEntry {
        id: 101,
        name: "skincategory2",
    },
    StringTypeEntry {
        id: 102,
        name: "skincategory3",
    },
    StringTypeEntry {
        id: 103,
        name: "skincategory4",
    },
    StringTypeEntry {
        id: 104,
        name: "skincategory5",
    },
    StringTypeEntry {
        id: 105,
        name: "skincategory6",
    },
    StringTypeEntry {
        id: 106,
        name: "skincategory7",
    },
    StringTypeEntry {
        id: 107,
        name: "skincategory8",
    },
    StringTypeEntry {
        id: 108,
        name: "skincategory9",
    },
    StringTypeEntry {
        id: 109,
        name: "skincategory10",
    },
    StringTypeEntry {
        id: 110,
        name: "skinitem1",
    },
    StringTypeEntry {
        id: 111,
        name: "skinitem2",
    },
    StringTypeEntry {
        id: 112,
        name: "skinitem3",
    },
    StringTypeEntry {
        id: 113,
        name: "skinitem4",
    },
    StringTypeEntry {
        id: 114,
        name: "skinitem5",
    },
    StringTypeEntry {
        id: 115,
        name: "skinitem6",
    },
    StringTypeEntry {
        id: 116,
        name: "skinitem7",
    },
    StringTypeEntry {
        id: 117,
        name: "skinitem8",
    },
    StringTypeEntry {
        id: 118,
        name: "skinitem9",
    },
    StringTypeEntry {
        id: 119,
        name: "skinitem10",
    },
    StringTypeEntry {
        id: 120,
        name: "rankingname1",
    },
    StringTypeEntry {
        id: 121,
        name: "rankingname2",
    },
    StringTypeEntry {
        id: 122,
        name: "rankingname3",
    },
    StringTypeEntry {
        id: 123,
        name: "rankingname4",
    },
    StringTypeEntry {
        id: 124,
        name: "rankingname5",
    },
    StringTypeEntry {
        id: 125,
        name: "rankingname6",
    },
    StringTypeEntry {
        id: 126,
        name: "rankingname7",
    },
    StringTypeEntry {
        id: 127,
        name: "rankingname8",
    },
    StringTypeEntry {
        id: 128,
        name: "rankingname9",
    },
    StringTypeEntry {
        id: 129,
        name: "rankingname10",
    },
    StringTypeEntry {
        id: 150,
        name: "coursetitle1",
    },
    StringTypeEntry {
        id: 151,
        name: "coursetitle2",
    },
    StringTypeEntry {
        id: 152,
        name: "coursetitle3",
    },
    StringTypeEntry {
        id: 153,
        name: "coursetitle4",
    },
    StringTypeEntry {
        id: 154,
        name: "coursetitle5",
    },
    StringTypeEntry {
        id: 155,
        name: "coursetitle6",
    },
    StringTypeEntry {
        id: 156,
        name: "coursetitle7",
    },
    StringTypeEntry {
        id: 157,
        name: "coursetitle8",
    },
    StringTypeEntry {
        id: 158,
        name: "coursetitle9",
    },
    StringTypeEntry {
        id: 159,
        name: "coursetitle10",
    },
    StringTypeEntry {
        id: 200,
        name: "targetnamep10",
    },
    StringTypeEntry {
        id: 201,
        name: "targetnamep9",
    },
    StringTypeEntry {
        id: 202,
        name: "targetnamep8",
    },
    StringTypeEntry {
        id: 203,
        name: "targetnamep7",
    },
    StringTypeEntry {
        id: 204,
        name: "targetnamep6",
    },
    StringTypeEntry {
        id: 205,
        name: "targetnamep5",
    },
    StringTypeEntry {
        id: 206,
        name: "targetnamep4",
    },
    StringTypeEntry {
        id: 207,
        name: "targetnamep3",
    },
    StringTypeEntry {
        id: 208,
        name: "targetnamep2",
    },
    StringTypeEntry {
        id: 209,
        name: "targetnamep1",
    },
    StringTypeEntry {
        id: 210,
        name: "targetnamen1",
    },
    StringTypeEntry {
        id: 211,
        name: "targetnamen2",
    },
    StringTypeEntry {
        id: 212,
        name: "targetnamen3",
    },
    StringTypeEntry {
        id: 213,
        name: "targetnamen4",
    },
    StringTypeEntry {
        id: 214,
        name: "targetnamen5",
    },
    StringTypeEntry {
        id: 215,
        name: "targetnamen6",
    },
    StringTypeEntry {
        id: 216,
        name: "targetnamen7",
    },
    StringTypeEntry {
        id: 217,
        name: "targetnamen8",
    },
    StringTypeEntry {
        id: 218,
        name: "targetnamen9",
    },
    StringTypeEntry {
        id: 219,
        name: "targetnamen10",
    },
    StringTypeEntry {
        id: 1000,
        name: "directory",
    },
    StringTypeEntry {
        id: 1001,
        name: "tablename",
    },
    StringTypeEntry {
        id: 1002,
        name: "tablelevel",
    },
    StringTypeEntry {
        id: 1003,
        name: "tablefull",
    },
    StringTypeEntry {
        id: 1010,
        name: "version",
    },
    StringTypeEntry {
        id: 1020,
        name: "irname",
    },
    StringTypeEntry {
        id: 1021,
        name: "irUserName",
    },
    StringTypeEntry {
        id: 1030,
        name: "songhashmd5",
    },
    StringTypeEntry {
        id: 1031,
        name: "songhashsha256",
    },
];

/// Delegate StringProperty that reads values from MainState::string_value().
/// This enables both StaticStateProvider (golden-master) and real game states
/// to provide string values through the same interface.
struct DelegateStringProperty {
    id: i32,
}

impl StringProperty for DelegateStringProperty {
    fn get(&self, state: &dyn MainState) -> String {
        state.string_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}
