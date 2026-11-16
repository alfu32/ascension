-- bld.airfield@industrial: Airfield (mesh.airfield_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.airfield_industrial','Airfield');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.airfield_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('field','plinth','mat.turf',0,'','{"w":5.6,"d":5.6,"h":0.03,"steps":1,"tint":"#a8b890"}'),
  ('runway','box','mat.tarmac',0,'','{"z":1.35,"w":5.5,"d":1.3,"h":0.04}'),
  ('runway_marks','box','mat.canvas',0,'','{"x":-2.25,"y":0.04,"z":1.35,"w":0.4,"d":0.06,"h":0.01,"repeat":6,"dx":0.9,"far":0}'),
  ('threshold','box','mat.cloth',1,'','{"x":-2.55,"y":0.04,"z":1.35,"w":0.18,"d":1.1,"h":0.01,"repeat":2,"dx":5.1}'),
  ('apron','box','mat.tarmac',0,'','{"x":-1,"z":-0.15,"w":2.8,"d":1.4,"h":0.03,"tint":"#d0d0d0"}'),
  ('hangar','cylinder','mat.corrugated',0,'','{"x":-1,"y":0,"z":-2.6,"w":2.4,"h":2.4,"segments":12,"pitch":90,"bottom":1}'),
  ('hangar_back','box','mat.red_brick',0,'','{"x":-1,"z":-2.62,"w":2.4,"d":0.06,"h":0.3}'),
  ('hangar_doors','box','mat.cloth',1,'','{"x":-1,"y":0,"z":-0.16,"w":1.9,"d":0.04,"h":0.95}'),
  ('hangar_sign','box','mat.cloth',1,'','{"x":-1,"y":0.95,"z":-0.16,"w":1.5,"d":0.04,"h":0.2}'),
  ('tower','box','mat.red_brick',0,'','{"x":1.5,"z":-1.4,"w":0.9,"d":0.9,"h":1.3,"door":0.28,"door_h":0.55,"windows":2,"window_y":0.75,"window_w":0.16,"window_h":0.3,"opening":"#22282f"}'),
  ('cab','box','mat.glass',0,'','{"x":1.5,"y":1.3,"z":-1.4,"w":0.8,"d":0.8,"h":0.45}'),
  ('cab_roof','flat_roof','mat.cloth',1,'','{"x":1.5,"y":1.75,"z":-1.4,"w":0.8,"d":0.8,"t":0.06,"overhang":0.08}'),
  ('balcony','fence','mat.cast_iron',0,'','{"x":1.5,"y":1.3,"z":-1.4,"w":1,"d":1,"h":0.2,"spacing":0.12,"rails":1,"ring":1,"far":0}'),
  ('mast','cylinder','mat.steel',0,'','{"x":1.5,"y":1.81,"z":-1.4,"w":0.03,"h":0.8,"segments":3,"far":0}'),
  ('sock_pole','cylinder','mat.cast_iron',0,'','{"x":2.45,"z":0.25,"w":0.05,"h":1.5,"segments":4}'),
  ('sock','cone','mat.cloth',1,'','{"x":2.45,"y":1.4,"z":0.12,"w":0.2,"h":0.6,"segments":6,"pitch":-100}'),
  ('fuel','cylinder','mat.rust',0,'','{"x":0.9,"z":-0.2,"w":0.4,"h":0.5,"segments":8}'),
  ('fuel_band','cylinder','mat.cloth',1,'','{"x":0.9,"y":0.32,"z":-0.2,"w":0.42,"h":0.1,"segments":8,"far":0}'),
  ('drums','cylinder','mat.rust',0,'','{"x":1.5,"z":-0.5,"w":0.2,"h":0.28,"segments":6,"repeat":3,"dx":0.24,"far":0}'),
  ('sheds','box','mat.sawn_timber',0,'','{"x":2.3,"z":-2.35,"w":0.8,"d":0.7,"h":0.6}'),
  ('shed_roof','gable_roof','mat.corrugated',0,'','{"x":2.3,"y":0.6,"z":-2.35,"w":0.8,"d":0.7,"pitch":20,"overhang":0.05,"t":0.03}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.airfield_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.airfield'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.airfield_industrial'), (SELECT id FROM def WHERE key='mat.turf'), 100);
