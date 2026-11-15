-- bld.dock@medieval: Medieval dock (mesh.dock_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_medieval','Medieval dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1}'),
  ('shed_back','box','mat.rough_stone',0,'','{"x":-0.85,"z":-1.58,"yaw":90,"w":0.16,"d":1.6,"h":1.25,"gable":46}'),
  ('shed_wall','wall','mat.rough_stone',0,'','{"x":-1.58,"z":-0.15,"yaw":90,"w":3,"h":1.25,"t":0.16,"windows":3,"window_y":0.65,"window_w":0.12,"window_h":0.3}'),
  ('shed_posts','box','mat.oak',0,'','{"x":-0.12,"z":-1.5,"w":0.14,"d":0.14,"h":1.25,"repeat":3,"dz":1.4}'),
  ('shed_beam','box','mat.oak',0,'','{"x":-0.12,"y":1.18,"z":-0.15,"w":0.16,"d":3,"h":0.12}'),
  ('shed_roof','gable_roof','mat.slate',0,'','{"x":-0.85,"y":1.25,"z":-0.15,"yaw":90,"w":3,"d":1.6,"pitch":46,"overhang":0.16,"t":0.08}'),
  ('shed_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.85,"y":1.935,"z":-0.15,"yaw":90,"w":3.36,"d":0.5,"pitch":46,"overhang":0,"t":0.03}'),
  ('cog','hull','mat.timber',0,'','{"x":-0.85,"y":0.2,"z":0.3,"yaw":90,"w":2.3,"d":0.95,"h":0.55,"sheer":0.2}'),
  ('cog_stripe','box','mat.cloth',1,'','{"x":-0.85,"y":0.66,"z":0.3,"yaw":90,"w":1.3,"d":0.99,"h":0.1}'),
  ('cog_castle','box','mat.sawn_timber',0,'','{"x":-0.85,"y":0.62,"z":-0.55,"w":0.6,"d":0.5,"h":0.32}'),
  ('chocks','box','mat.oak',0,'','{"x":-0.85,"z":-0.3,"w":0.6,"d":0.15,"h":0.22,"repeat":3,"dz":0.6,"far":0}'),
  ('quay','box','mat.timber',0,'','{"x":0.95,"y":0.4,"w":1.1,"d":3.5,"h":0.1}'),
  ('quay_beams','box','mat.oak',0,'','{"x":0.95,"y":0.28,"w":0.9,"d":3.4,"h":0.12}'),
  ('quay_posts','cylinder','mat.oak',0,'','{"x":0.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067}'),
  ('quay_posts_e','cylinder','mat.oak',0,'','{"x":1.45,"z":-1.6,"w":0.15,"h":0.8,"segments":6,"repeat":4,"dz":1.067,"far":0}'),
  ('crane_house','box','mat.daub',0,'','{"x":0.95,"y":0.5,"z":-1.1,"w":0.9,"d":0.9,"h":1}'),
  ('crane_frame','box','mat.oak',0,'','{"x":0.5,"y":0.5,"z":-1.1,"w":0.06,"d":0.93,"h":1,"repeat":3,"dx":0.45,"far":0}'),
  ('crane_roof','gable_roof','mat.shingle',0,'','{"x":0.95,"y":1.5,"z":-1.1,"yaw":90,"w":0.9,"d":0.9,"pitch":42,"overhang":0.1,"t":0.08}'),
  ('crane_roof_cap','gable_roof','mat.cloth',1,'','{"x":0.95,"y":1.851,"z":-1.1,"yaw":90,"w":1.14,"d":0.36,"pitch":42,"overhang":0,"t":0.03}'),
  ('wheel','cylinder','mat.oak',0,'','{"x":0.45,"y":1.05,"z":-1.1,"w":1.1,"h":0.16,"segments":10,"roll":90,"bottom":1}'),
  ('jib','box','mat.oak',0,'','{"x":0.95,"y":1.7,"z":-0.75,"w":0.12,"d":0.12,"h":1.35,"pitch":58}'),
  ('rope','cylinder','mat.thatch',0,'','{"x":0.95,"y":1.25,"z":0.38,"w":0.04,"h":1.2,"segments":4,"far":0}'),
  ('load','box','mat.timber',0,'','{"x":0.95,"y":0.95,"z":0.38,"w":0.4,"d":0.4,"h":0.36}'),
  ('steps','plinth','mat.timber',0,'','{"x":0.28,"z":0.6,"w":0.36,"d":0.6,"h":0.45,"steps":3,"inset":0.06}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":0.75,"y":0.5,"z":1,"w":0.24,"h":0.34,"segments":7,"repeat":2,"dz":0.3,"far":0,"tint":"#c8a888"}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.4,"y":0.5,"z":1.55,"h":2.5,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.dock_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
