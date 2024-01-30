pub const LOAD_STAGE: &'static str = r#"
  FOR scene IN @@scene_collection
    FILTER scene._key == @scene_key
    LET occupants = (FOR v, edge IN OUTBOUND scene._id GRAPH 'world'
      FILTER edge.relation == "scene-has-person"
      RETURN v)

    LET items = (FOR v, edge IN OUTBOUND scene._id GRAPH 'world'
      FILTER edge.relation == "item-located-at"
      RETURN v)

    LET exits = (FOR v, edge in OUTBOUND scene._id GRAPH 'world'
      FILTER edge.relation == "connects-to"
      FOR exit in scene.exits || [] // Stubs have no exits field
        FILTER exit.scene_key == v._key
      RETURN MERGE(exit, { scene_id: v._id }))

    RETURN {
      "id": scene._id,
      "key": scene._key,
      "scene": MERGE(scene, { "exits": exits }),
      "people": occupants,
      "items": items,
    }
"#;

pub const LOAD_ENTITY: &'static str = r#"
LET entities = (
    FOR scene IN @@scene_collection
        FILTER scene._key == @scene_key
        LET occupants = (FOR v, edge IN OUTBOUND scene._id GRAPH 'world'
          FILTER edge.relation == "scene-has-person" and v._key == @entity_key
          RETURN MERGE({ "type": "Person"}, v))

        LET items = (FOR v, edge IN OUTBOUND scene._id GRAPH 'world'
          FILTER edge.relation == "item-located-at" and v._key == @entity_key
          RETURN MERGE({ "type": "Item" }, v ))

        RETURN FIRST(APPEND(occupants, items)))

FOR ent in entities
    FILTER ent != null
RETURN ent
"#;

pub const UPSERT_SCENE: &'static str = r#"
  UPSERT { _key: @scene_key }
    INSERT <SCENE_JSON>
    UPDATE <SCENE_JSON>
  IN @@scene_collection
    RETURN { "_id": NEW._id, "_key": NEW._key }
"#;

pub const LOAD_CACHED_COMMAND: &'static str = r#"
  FOR cmd IN @@cache_collection
    FILTER cmd.raw == @raw_cmd && cmd.scene_key == @scene_key
    RETURN cmd
"#;
