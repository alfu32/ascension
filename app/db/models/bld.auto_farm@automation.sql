-- bld.auto_farm@automation: Automated Farm (mesh.auto_farm_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_farm_automation','Automated Farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_farm_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('tray_1','box','mat.white_composite',0,'','{"x":-0.55,"y":0.03,"z":-0.55,"w":2.4,"d":2.2,"h":0.08}'),
  ('crops_1','box','mat.greens',0,'','{"x":-0.55,"y":0.11,"z":-0.55,"w":2.2,"d":2,"h":0.14}'),
  ('grow_1','box','mat.grow_light',0,'','{"x":-0.55,"y":0.55,"z":-0.55,"w":2.1,"d":1.9,"h":0.03,"far":0}'),
  ('tray_2','box','mat.white_composite',1,'','{"x":-0.55,"y":0.63,"z":-0.55,"w":2.4,"d":2.2,"h":0.08}'),
  ('crops_2','box','mat.greens',0,'','{"x":-0.55,"y":0.71,"z":-0.55,"w":2.2,"d":2,"h":0.14}'),
  ('grow_2','box','mat.grow_light',0,'','{"x":-0.55,"y":1.15,"z":-0.55,"w":2.1,"d":1.9,"h":0.03,"far":0}'),
  ('tray_3','box','mat.white_composite',1,'','{"x":-0.55,"y":1.23,"z":-0.55,"w":2.4,"d":2.2,"h":0.08}'),
  ('crops_3','box','mat.greens',0,'','{"x":-0.55,"y":1.31,"z":-0.55,"w":2.2,"d":2,"h":0.14}'),
  ('grow_3','box','mat.grow_light',0,'','{"x":-0.55,"y":1.75,"z":-0.55,"w":2.1,"d":1.9,"h":0.03,"far":0}'),
  ('tray_4','box','mat.white_composite',1,'','{"x":-0.55,"y":1.83,"z":-0.55,"w":2.4,"d":2.2,"h":0.08}'),
  ('crops_4','box','mat.greens',0,'','{"x":-0.55,"y":1.91,"z":-0.55,"w":2.2,"d":2,"h":0.14}'),
  ('grow_4','box','mat.grow_light',0,'','{"x":-0.55,"y":2.35,"z":-0.55,"w":2.1,"d":1.9,"h":0.03,"far":0}'),
  ('posts','box','mat.white_composite',0,'','{"x":-1.7,"y":0.03,"z":-1.6,"w":0.12,"d":0.12,"h":2.5,"repeat":2,"dx":2.3}'),
  ('posts_s','box','mat.white_composite',0,'','{"x":-1.7,"y":0.03,"z":0.5,"w":0.12,"d":0.12,"h":2.5,"repeat":2,"dx":2.3}'),
  ('roof_band','box','mat.white_composite',1,'','{"x":-0.55,"y":2.43,"z":-0.55,"w":2.5,"d":2.3,"h":0.16}'),
  ('roof','flat_roof','mat.white_composite',0,'','{"x":-0.55,"y":2.59,"z":-0.55,"w":2.5,"d":2.3,"t":0.04,"overhang":0}'),
  ('roof_solar','box','mat.solar_panel',0,'','{"x":-0.55,"y":2.66,"z":-1.1,"w":2.2,"d":0.8,"h":0.03,"pitch":18,"repeat":2,"dz":1,"far":0}'),
  ('beds','box','mat.white_composite',0,'','{"x":1.2,"y":0.03,"z":-0.2,"w":0.32,"d":3.2,"h":0.1,"repeat":2,"dx":0.5}'),
  ('bed_crops','box','mat.greens',0,'','{"x":1.2,"y":0.13,"z":-0.2,"w":0.26,"d":3.1,"h":0.14,"repeat":2,"dx":0.5}'),
  ('robot_legs','box','mat.white_composite',0,'','{"x":0.92,"y":0.03,"z":0.55,"w":0.07,"d":0.3,"h":0.5,"repeat":2,"dx":0.98}'),
  ('robot','box','mat.white_composite',1,'','{"x":1.41,"y":0.53,"z":0.55,"w":1.08,"d":0.34,"h":0.14}'),
  ('robot_arm','box','mat.graphite',0,'','{"x":1.2,"y":0.3,"z":0.55,"w":0.08,"d":0.08,"h":0.24,"far":0}'),
  ('tanks','cylinder','mat.white_composite',0,'','{"x":-1.4,"y":0.03,"z":1.25,"w":0.6,"h":0.75,"segments":12,"repeat":2,"dx":0.75}'),
  ('tank_tops','dome','mat.white_composite',0,'','{"x":-1.4,"y":0.78,"z":1.25,"w":0.6,"h":0.2,"segments":12,"repeat":2,"dx":0.75,"far":0}'),
  ('tank_bands','cylinder','mat.white_composite',1,'','{"x":-1.4,"y":0.4,"z":1.25,"w":0.62,"h":0.24,"segments":12,"repeat":2,"dx":0.75}'),
  ('pipe','box','mat.graphite',0,'','{"x":-1.05,"y":0.2,"z":0.75,"w":0.06,"d":0.6,"h":0.06,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.35,"y":0.03,"z":1.5,"h":2,"fw":0.45,"fh":0.32,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_farm_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_farm'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_farm_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
