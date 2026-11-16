-- bld.farm@modern: Modern farm (mesh.farm_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_modern','Modern farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('barn','box','mat.cladding',0,'','{"x":-1,"y":0.03,"z":-1.2,"w":1.6,"d":1.1,"h":0.95,"gable":18,"door":0.6,"door_h":0.72,"door_colour":"#d8d8d8","tint":"#d07862"}'),
  ('roof','gable_roof','mat.white_metal',0,'','{"x":-1,"y":0.98,"z":-1.2,"w":1.6,"d":1.1,"pitch":18,"overhang":0.1,"t":0.05}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-1,"y":1.1138,"z":-1.2,"w":1.84,"d":0.6,"pitch":18,"overhang":0,"t":0.03}'),
  ('shed_posts','cylinder','mat.steel',0,'','{"x":-0.1,"y":0.03,"z":-0.7,"w":0.06,"h":1,"segments":4,"repeat":3,"dx":0.5}'),
  ('shed_wall','box','mat.cladding',0,'','{"x":0.4,"y":0.03,"z":-1.72,"w":1.1,"d":0.04,"h":0.95}'),
  ('shed_roof','flat_roof','mat.cladding',0,'','{"x":0.4,"y":1.03,"z":-1.2,"w":1.1,"d":1.1,"t":0.05,"overhang":0.06}'),
  ('tractor','box','mat.white_metal',0,'','{"x":0.4,"y":0.2,"z":-1.15,"w":0.28,"d":0.62,"h":0.2,"tint":"#4f9a40"}'),
  ('tractor_cab','box','mat.curtain_wall',0,'','{"x":0.4,"y":0.4,"z":-1.32,"w":0.3,"d":0.28,"h":0.3}'),
  ('tractor_wheels','cylinder','mat.rubber',0,'','{"x":0.18,"y":0.22,"z":-1.32,"w":0.42,"h":0.08,"segments":8,"roll":90,"repeat":2,"dx":0.36}'),
  ('tractor_front','cylinder','mat.rubber',0,'','{"x":0.21,"y":0.13,"z":-0.95,"w":0.24,"h":0.06,"segments":6,"roll":90,"repeat":2,"dx":0.32,"far":0}'),
  ('silo','include',NULL,0,'','{"mesh":"mesh.silo_modern","x":1.4,"y":0.03,"z":-1.35,"scale":0.85}'),
  ('bins','cylinder','mat.corrugated',0,'','{"x":1.4,"y":0.03,"z":-0.25,"w":0.7,"h":0.75,"segments":10}'),
  ('bins_roof','cone','mat.white_metal',0,'','{"x":1.4,"y":0.78,"z":-0.25,"w":0.76,"h":0.28,"segments":10}'),
  ('bales','cylinder','mat.thatch',0,'','{"x":-1.65,"y":0.2,"z":-0.3,"w":0.36,"h":0.32,"segments":8,"roll":90,"repeat":3,"dx":0.4}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.3,"y":0.03,"z":-0.45,"h":1.9,"fw":0.45,"fh":0.32}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.farm_modern'), (SELECT id FROM def WHERE key='mat.cladding'), 100);
