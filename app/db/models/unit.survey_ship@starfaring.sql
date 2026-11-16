-- unit.survey_ship@starfaring: Survey Ship (mesh.survey_ship_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.survey_ship_starfaring','Survey Ship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.survey_ship_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','cylinder','mat.steel',0,'','{"y":0.34,"z":-0.8,"w":0.36,"h":1.35,"segments":8,"pitch":90,"tint":"#b4bcc8"}'),
  ('nose','cone','mat.steel',0,'','{"y":0.34,"z":0.55,"w":0.36,"h":0.32,"segments":8,"pitch":90,"tint":"#c3cbd6"}'),
  ('band','cylinder','mat.steel',1,'','{"y":0.34,"z":-0.25,"w":0.4,"h":0.3,"segments":8,"pitch":90}'),
  ('boom','cylinder','mat.iron',0,'','{"y":0.34,"z":0.8,"w":0.05,"h":0.5,"segments":4,"pitch":90,"tint":"#7c8592"}'),
  ('probe','cone','mat.steel',1,'','{"y":0.34,"z":1.3,"w":0.12,"h":0.16,"segments":5,"pitch":90}'),
  ('dish_post','cylinder','mat.iron',0,'','{"y":0.5,"z":0.25,"w":0.06,"h":0.22,"segments":4,"tint":"#7c8592"}'),
  ('dish','dome','mat.white_metal',0,'','{"y":0.72,"z":0.25,"w":0.85,"h":0.1,"segments":12,"pitch":30}'),
  ('dish_back','dome','mat.white_metal',0,'','{"y":0.72,"z":0.25,"w":0.85,"h":0.2,"segments":12,"pitch":-150,"far":0}'),
  ('feed','cylinder','mat.iron',0,'','{"y":0.72,"z":0.25,"w":0.04,"h":0.3,"segments":4,"pitch":30,"far":0}'),
  ('panels','box','mat.solar_panel',0,'','{"x":-0.66,"y":0.32,"z":-0.5,"w":0.82,"d":0.4,"h":0.03,"repeat":2,"dx":1.32}'),
  ('struts','box','mat.steel',1,'','{"x":-0.66,"y":0.31,"z":-0.5,"w":0.9,"d":0.08,"h":0.05,"repeat":2,"dx":1.32}'),
  ('fin','box','mat.steel',1,'','{"y":0.48,"z":-0.75,"w":0.04,"d":0.38,"h":0.24,"taper":0.5}'),
  ('engines','cylinder','mat.iron',0,'','{"x":-0.22,"y":0.3,"z":-1.05,"w":0.18,"h":0.36,"segments":6,"pitch":90,"repeat":2,"dx":0.44,"tint":"#4a5260"}'),
  ('drives','wheel','mat.glow',0,'','{"x":-0.22,"y":0.3,"z":-1.07,"yaw":90,"w":0.15,"t":0.04,"spokes":0,"segments":6,"repeat":2,"dx":0.44}'),
  ('emitter','cylinder','mat.iron',0,'','{"y":0.2,"z":0.45,"w":0.08,"h":0.3,"segments":4,"pitch":90,"tint":"#4a5260","far":0}'),
  ('spark','cone','mat.flash',0,'','{"y":0.2,"z":0.77,"w":0.14,"h":0.14,"segments":3,"pitch":90,"flash":1,"far":0}'),
  ('bolt','cylinder','mat.light_strip',0,'spark','{"y":0.2,"w":0.05,"h":0.6,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.survey_ship_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.survey_ship'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.survey_ship_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
