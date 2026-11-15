-- bld.barracks@ancient: Early barracks (mesh.barracks_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_ancient','Early barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('palisade','palisade','mat.timber',0,'','{"w":3.6,"d":3.6,"h":1.2,"t":0.16,"ring":1,"gap":1.1,"spacing":0.24}'),
  ('gate_posts','cylinder','mat.timber',0,'','{"x":-0.62,"z":1.72,"w":0.2,"h":1.75,"segments":6,"repeat":2,"dx":1.24}'),
  ('gate_beam','box','mat.timber',0,'','{"y":1.55,"z":1.72,"w":1.6,"d":0.18,"h":0.15}'),
  ('gate_cloth','box','mat.cloth',1,'','{"y":1.2,"z":1.84,"w":1.15,"d":0.04,"h":0.34}'),
  ('hut','box','mat.mud_plaster',0,'','{"z":-0.85,"w":2.3,"d":1.25,"h":1.05,"gable":38,"door":0.38,"door_h":0.7,"windows":2,"window_y":0.55,"window_w":0.18,"window_h":0.2}'),
  ('hut_roof','gable_roof','mat.thatch',0,'','{"y":1.05,"z":-0.85,"w":2.3,"d":1.25,"pitch":38,"overhang":0.18,"t":0.1}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.15,"z":0.95,"h":3.6,"fw":0.75,"fh":1.3,"notch":0.25}'),
  ('rack','fence','mat.timber',0,'','{"x":-1.1,"z":0.75,"w":1.0,"h":0.55,"spacing":0.5,"rails":1}'),
  ('spears','cylinder','mat.timber',0,'','{"x":-1.43,"z":0.7,"w":0.045,"h":1.45,"segments":4,"repeat":4,"dx":0.22}'),
  ('dummy','cylinder','mat.timber',0,'','{"x":0.2,"z":0.6,"w":0.1,"h":1.1,"segments":5}'),
  ('dummy_body','box','mat.thatch',0,'','{"x":0.2,"y":0.55,"z":0.6,"w":0.36,"d":0.26,"h":0.45,"taper":0.25}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.barracks_ancient'), (SELECT id FROM def WHERE key='mat.mud_plaster'), 100);
